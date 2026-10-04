//! Engine-owned payload installation and measured tool availability.
use crate::{
    Environment, Error, Options, Result, Store, access,
    json::OpaqueObject,
    persist,
    runtime::{self, Processes, SpawnOptions},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
const PREFIX: &str = "/opt/workflow/tools";
const CLAUDE: &str = "/opt/workflow/tools/claude/bin/claude";
/// Members every payload carries. Codex resolves `codex-code-mode-host` beside its own executable;
/// without it Code Mode fails closed.
const REQUIRED: [&str; 4] = [
    "codex/bin/codex",
    "codex/bin/codex-code-mode-host",
    "notices/codex-LICENSE",
    "notices/claude-code-LICENSE",
];
/// Host directory of the visible, read-only agent tools below `.workspace/`.
fn agent_tools(opts: &Options) -> PathBuf {
    opts.root.join(access::DIRECTORY).join(access::AGENT_TOOLS)
}
static VERIFIED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
static STATES: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<ToolState>>>>> = OnceLock::new();
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ToolCatalog {
    format: u64,
    architecture: String,
    size: u64,
    sha256: String,
    files: Vec<ToolFile>,
    tools: Vec<ToolDefinition>,
    optional: OptionalTools,
    #[serde(flatten)]
    extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct ToolFile {
    path: String,
    size: u64,
    sha256: String,
    executable: bool,
    #[serde(flatten)]
    extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct ToolDefinition {
    id: String,
    version: String,
    binary: String,
    #[serde(flatten)]
    extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct OptionalTools {
    claude: ClaudePin,
    #[serde(flatten)]
    extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct ClaudePin {
    version: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    bytes: Option<u64>,
    #[serde(flatten)]
    extra: OpaqueObject,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolPhase {
    NotInstalled,
    Installing,
    Verifying,
    Ready,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub architecture: Option<String>,
    #[serde(default)]
    pub binary: Option<String>,
    pub phase: ToolPhase,
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl ToolStatus {
    fn absent(id: &str, binary: &str, version: Option<String>) -> Self {
        Self {
            id: id.into(),
            version,
            architecture: Some(architecture().into()),
            binary: Some(binary.into()),
            phase: ToolPhase::NotInstalled,
            progress: None,
            error: None,
            operation_id: None,
            extra: OpaqueObject::new(),
        }
    }
    fn measured(&mut self, result: Result<()>) {
        self.phase = if result.is_ok() {
            ToolPhase::Ready
        } else {
            ToolPhase::Failed
        };
        self.error = result.err().map(|e| e.message);
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ToolsStatus {
    pub revision: u64,
    pub tools: Vec<ToolStatus>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
struct ToolState {
    revision: u64,
    tools: Vec<ToolStatus>,
    #[serde(default)]
    measured: Option<(String, String)>,
    #[serde(flatten)]
    extra: OpaqueObject,
}
impl ToolState {
    fn projection(&self) -> ToolsStatus {
        ToolsStatus {
            revision: self.revision,
            tools: self.tools.clone(),
        }
    }
}
fn err(message: &str) -> Error {
    Error::business("invalid_tools", message)
}
fn architecture() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "amd64"
    }
}
fn hash_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut hash = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        hash.update(&b[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
fn member(name: &str) -> Result<&Path> {
    let p = Path::new(name);
    if name.is_empty()
        || name.contains('\\')
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || p.components().any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(err("unsafe tools member"));
    }
    Ok(p)
}
fn source(opts: &Options) -> Result<(PathBuf, ToolCatalog)> {
    if let Some(dir) = &opts.tools {
        return Ok((
            dir.join("tools.zip"),
            persist::read_json(&dir.join("tools.json"))?,
        ));
    }
    let apk = opts
        .apk
        .as_ref()
        .ok_or_else(|| err("Engine tool payload is not configured"))?;
    let mut z =
        zip::ZipArchive::new(File::open(apk)?).map_err(|_| err("invalid APK tool payload"))?;
    let metadata: ToolCatalog = {
        let mut file = z
            .by_name("assets/environment/tools/tools.json")
            .map_err(|_| err("missing tools catalog"))?;
        if file.size() > 4 * 1024 * 1024 {
            return Err(err("tools catalog too large"));
        }
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        crate::json::strict_json(&bytes).map_err(|_| err("invalid tools catalog"))?
    };
    let sha = &metadata.sha256;
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("invalid payload digest"));
    }
    let dir = opts.root.join(".workspace/environment/tools/archives");
    fs::create_dir_all(&dir)?;
    let archive = dir.join(format!("{sha}.zip"));
    if !archive.exists() {
        let mut file = z
            .by_name("assets/environment/tools/tools.zip")
            .map_err(|_| err("missing tools archive"))?;
        if file.size() > 1024 * 1024 * 1024 || file.size() != metadata.size {
            return Err(err("invalid payload size"));
        }
        let part = dir.join(format!(".{}", uuid::Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut dest = File::create(&part)?;
            std::io::copy(&mut file, &mut dest)?;
            dest.sync_all()?;
            if hash_file(&part)? != *sha {
                return Err(err("payload checksum mismatch"));
            }
            fs::rename(&part, &archive)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&part);
        }
        result?;
    }
    Ok((archive, metadata))
}
fn extract(archive: &Path, catalog: &ToolCatalog, target: &Path) -> Result<()> {
    if catalog.format != 1 || catalog.architecture != architecture() {
        return Err(err("unsupported tools format or architecture"));
    }
    if catalog.size != fs::metadata(archive)?.len() || catalog.sha256 != hash_file(archive)? {
        return Err(err("payload checksum or size mismatch"));
    }
    let entries = &catalog.files;
    if entries.len() > 10000 {
        return Err(err("too many tool members"));
    }
    let mut z =
        zip::ZipArchive::new(File::open(archive)?).map_err(|_| err("invalid tools archive"))?;
    if z.len() != entries.len() {
        return Err(err("unexpected tools archive members"));
    }
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for item in entries {
        let name = item.path.as_str();
        let relative = member(name)?;
        if !seen.insert(name) {
            return Err(err("duplicate tool member"));
        }
        let mut src = z.by_name(name).map_err(|_| err("missing tool member"))?;
        if src.is_dir()
            || src
                .unix_mode()
                .is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
        {
            return Err(err("tool members must be regular files"));
        }
        total = total
            .checked_add(src.size())
            .ok_or_else(|| err("tools size overflow"))?;
        if total > 2 * 1024 * 1024 * 1024 || src.size() != item.size {
            return Err(err("tool member size mismatch"));
        }
        let path = target.join(relative);
        fs::create_dir_all(path.parent().unwrap())?;
        let mut dest = File::create(&path)?;
        std::io::copy(&mut src, &mut dest)?;
        dest.sync_all()?;
        if item.sha256 != hash_file(&path)? {
            return Err(err("tool member checksum mismatch"));
        }
        fs::set_permissions(
            &path,
            fs::Permissions::from_mode(if item.executable { 0o755 } else { 0o644 }),
        )?;
    }
    for required in REQUIRED {
        if !seen.contains(required) {
            return Err(err("incomplete required tools payload"));
        }
    }
    Ok(())
}
fn validate_catalog(catalog: &ToolCatalog) -> Result<()> {
    if catalog.format != 1 || catalog.architecture != architecture() {
        return Err(err("unsupported tools format or architecture"));
    }
    let files = &catalog.files;
    if files.len() > 10000 {
        return Err(err("too many tool members"));
    }
    let mut seen = HashSet::new();
    for file in files {
        let name = file.path.as_str();
        member(name)?;
        let sha = file.sha256.as_str();
        if !seen.insert(name) || sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(err("invalid tools inventory"));
        }
    }
    for path in REQUIRED {
        if !seen.contains(path) {
            return Err(err("missing mandatory tool artifact"));
        }
    }
    let tools = &catalog.tools;
    if tools.len() != 1 {
        return Err(err("invalid mandatory tools"));
    }
    for (id, binary) in [("codex", "/opt/workflow/tools/codex/bin/codex")] {
        if !tools
            .iter()
            .any(|t| t.id == id && t.binary == binary && !t.version.is_empty())
        {
            return Err(err("invalid mandatory tool entry"));
        }
    }
    Ok(())
}
fn prepare(opts: &Options) -> Result<(PathBuf, ToolCatalog)> {
    let (archive, catalog) = source(opts)?;
    validate_catalog(&catalog)?;
    let digest = catalog.sha256.as_str();
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("invalid payload digest"));
    }
    let target = agent_tools(opts).join("payload").join(digest);
    let mut verified = VERIFIED.get_or_init(Default::default).lock().unwrap();
    if !verified.contains(&target) {
        let valid = target.is_dir()
            && catalog.files.iter().all(|item| {
                member(&item.path).ok().is_some_and(|relative| {
                    let path = target.join(relative);
                    fs::symlink_metadata(&path)
                        .is_ok_and(|m| m.file_type().is_file() && m.len() == item.size)
                        && hash_file(&path).is_ok_and(|hash| item.sha256 == hash)
                })
            });
        if !valid {
            if target.exists() {
                return Err(err("installed tool payload failed verification"));
            }
            fs::create_dir_all(target.parent().unwrap())?;
            let stage = target.with_file_name(format!(".{}", uuid::Uuid::new_v4()));
            fs::create_dir(&stage)?;
            let result = extract(&archive, &catalog, &stage).and_then(|_| {
                fs::rename(&stage, &target)?;
                Ok(())
            });
            if result.is_err() {
                let _ = fs::remove_dir_all(&stage);
            }
            result?;
        }
        verified.insert(target.clone());
    }
    Ok((target, catalog))
}
/// Required executable checks are also run against candidate generations before activation.
pub fn verification_commands(opts: &Options) -> Result<Vec<(Vec<String>, String)>> {
    if opts.tools.is_none() && opts.apk.is_none() {
        return Ok(vec![]);
    }
    let (_, catalog) = prepare(opts)?;
    let mut checks = vec![];
    for tool in &catalog.tools {
        checks.push((
            vec![tool.binary.clone(), "--version".into()],
            tool.version.clone(),
        ));
    }
    Ok(checks)
}
/// Apply the current payload for every generation without changing its filesystem or post-scripts.
pub(crate) fn bind(opts: &Options, command: &mut Command) -> Result<()> {
    if opts.tools.is_none() && opts.apk.is_none() {
        return Ok(());
    }
    let (tree, catalog) = prepare(opts)?;
    command
        .arg("--bind")
        .arg(format!("{}:{PREFIX}", tree.display()));
    let launcher = agent_tools(opts).join("launchers/codex");
    let script = include_bytes!("../guest/codex");
    if fs::read(&launcher).ok().as_deref() != Some(script) {
        persist::atomic(&launcher, script)?;
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755))?;
    }
    command
        .arg("--bind")
        .arg(format!("{}:/usr/local/bin/codex", launcher.display()));
    let optional = optional_dir(opts, &catalog)?;
    fs::create_dir_all(optional.join("bin"))?;
    command
        .arg("--bind")
        .arg(format!("{}:{PREFIX}/claude", optional.display()));
    // The endpoint exists before optional installation; availability is measured separately.
    if optional.join("bin/claude").is_file() {
        command.arg("--bind").arg(format!(
            "{}:/usr/local/bin/claude",
            optional.join("bin/claude").display()
        ));
    }
    Ok(())
}
fn optional_dir(opts: &Options, catalog: &ToolCatalog) -> Result<PathBuf> {
    let version = catalog.optional.claude.version.as_str();
    persist::identifier(version)?;
    Ok(agent_tools(opts).join("claude").join(version))
}

fn state(opts: &Options) -> Result<Arc<Mutex<ToolState>>> {
    let mut states = STATES.get_or_init(Default::default).lock().unwrap();
    if let Some(state) = states.get(&opts.root) {
        return Ok(state.clone());
    }
    let store = Store::open(&opts.root)?;
    let mut value = match store.read::<ToolState>("environment/tools/state.json") {
        Ok(Some(value)) => value,
        Ok(None) => ToolState::default(),
        Err(_) => {
            store.quarantine("environment/tools/state.json")?;
            ToolState::default()
        }
    };
    value.measured = None;
    for tool in &mut value.tools {
        if matches!(tool.phase, ToolPhase::Installing | ToolPhase::Verifying) {
            tool.phase = ToolPhase::Failed;
            tool.error = Some("Installation interrupted; retry explicitly".into());
        }
    }
    let value = Arc::new(Mutex::new(value));
    states.insert(opts.root.clone(), value.clone());
    Ok(value)
}
fn persist(opts: &Options, state: &mut ToolState) -> Result<()> {
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or_else(|| Error::business("overflow", "tool revision overflow"))?;
    Store::open(&opts.root)?.write("environment/tools/state.json", state)
}
pub fn status(shared: &Arc<Mutex<Environment>>) -> Result<ToolsStatus> {
    let (opts, active) = {
        let env = shared.lock().unwrap();
        (env.options.clone(), env.state.active.clone())
    };
    let state = state(&opts)?;
    let mut state = state.lock().unwrap();
    let Some(active) = active else {
        let tools = [
            ("codex", "/opt/workflow/tools/codex/bin/codex"),
            ("claude", CLAUDE),
        ]
        .into_iter()
        .map(|(id, binary)| ToolStatus::absent(id, binary, None))
        .collect();
        return Ok(ToolsStatus {
            revision: state.revision,
            tools,
        });
    };
    let (_, catalog) = prepare(&opts)?;
    let identity = (catalog.sha256.clone(), active.generation);
    if state.measured.as_ref() != Some(&identity) {
        let previous = state.tools.iter().find(|t| t.id == "claude").cloned();
        let mut items = vec![];
        for definition in &catalog.tools {
            let mut tool = ToolStatus::absent(
                &definition.id,
                &definition.binary,
                Some(definition.version.clone()),
            );
            tool.extra = definition.extra.clone();
            let argv = vec![definition.binary.clone(), "--version".into()];
            tool.measured(runtime::probe(shared, argv, &definition.version));
            items.push(tool);
        }
        let pin = &catalog.optional.claude;
        let binary = optional_dir(&opts, &catalog)?.join("bin/claude");
        let mut claude = ToolStatus::absent("claude", CLAUDE, Some(pin.version.clone()));
        if let Some(old) = previous.filter(|t| {
            t.version.as_ref() == Some(&pin.version)
                && matches!(
                    t.phase,
                    ToolPhase::Failed | ToolPhase::Installing | ToolPhase::Verifying
                )
        }) {
            claude = old;
        } else if binary.is_file() {
            let result = if Some(hash_file(&binary)?) == pin.sha256 {
                runtime::probe(
                    shared,
                    vec![CLAUDE.into(), "--version".into()],
                    &pin.version,
                )
            } else {
                Err(err("Claude checksum mismatch"))
            };
            claude.measured(result);
        }
        items.push(claude);
        state.tools = items;
        state.measured = Some(identity);
        persist(&opts, &mut state)?;
    }
    Ok(state.projection())
}
pub fn install(
    shared: &Arc<Mutex<Environment>>,
    processes: Arc<Processes>,
    tool_id: &str,
    retry: bool,
) -> Result<ToolsStatus> {
    if tool_id != "claude" {
        return Err(Error::invalid("only optional Claude requires installation"));
    }
    let _ = status(shared)?;
    let opts = shared.lock().unwrap().options.clone();
    let (_, catalog) = prepare(&opts)?;
    let state = state(&opts)?;
    let mut locked = state.lock().unwrap();
    let tool = locked
        .tools
        .iter_mut()
        .find(|t| t.id == "claude")
        .ok_or_else(|| err("tool status unavailable"))?;
    if matches!(
        tool.phase,
        ToolPhase::Ready | ToolPhase::Installing | ToolPhase::Verifying
    ) || tool.phase == ToolPhase::Failed && !retry
    {
        return Ok(locked.projection());
    }
    tool.phase = ToolPhase::Installing;
    tool.operation_id = Some(uuid::Uuid::new_v4().to_string());
    tool.error = None;
    persist(&opts, &mut locked)?;
    let response = locked.projection();
    drop(locked);
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        let result = (|| -> Result<()> {
            let pin = &catalog.optional.claude;
            let url = pin
                .url
                .as_deref()
                .ok_or_else(|| err("missing Claude URL"))?;
            let sha = pin
                .sha256
                .as_deref()
                .ok_or_else(|| err("missing Claude digest"))?;
            if !url.starts_with("https://downloads.claude.ai/")
                || !url
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/:._-".contains(&b))
                || sha.len() != 64
                || !sha.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(err("invalid Claude release pin"));
            }
            let bytes = pin.bytes.ok_or_else(|| err("missing Claude byte size"))?;
            let script = format!(
                "set -eu\np={PREFIX}/claude/bin/claude.download\ncurl -fsSL --connect-timeout 20 --max-time 480 --retry 3 --retry-max-time 480 --max-filesize {bytes} -C - -o \"$p\" '{url}'\nif ! test \"$(wc -c < \"$p\")\" -eq {bytes}; then rm -f \"$p\"; exit 71; fi\nif ! printf '%s  %s\\n' '{sha}' \"$p\" | sha256sum -c --quiet; then rm -f \"$p\"; exit 72; fi\nchmod 0755 \"$p\"\n\"$p\" --version\nmv -f \"$p\" {CLAUDE}\n"
            );
            let spawned = processes.spawn(
                &shared,
                &SpawnOptions {
                    argv: vec!["/bin/bash".into(), "-c".into(), script],
                    cwd: "/home/work".into(),
                    ..SpawnOptions::default()
                },
            )?;
            let id = &spawned.process_id;
            let deadline = Instant::now() + Duration::from_secs(600);
            loop {
                let done = processes.wait(id, 1000)?;
                if !done.running {
                    if done.exit_code != Some(0) {
                        let code = done.exit_code.unwrap_or(-1);
                        let reason = match code {
                            28 => "Claude download timed out; retry resumes the partial download",
                            6 | 7 => "Claude download could not connect; retry explicitly",
                            22 => "Claude download server rejected the request; retry explicitly",
                            60 => "Claude download TLS certificate verification failed",
                            _ => "Claude download or verification failed",
                        };
                        return Err(err(&format!("{reason} (exit {code})")));
                    }
                    break;
                }
                if Instant::now() > deadline {
                    let _ = processes.stop(id, true);
                    return Err(err("Claude installation timed out"));
                }
            }
            {
                let mut s = state.lock().unwrap();
                if let Some(t) = s.tools.iter_mut().find(|t| t.id == "claude") {
                    t.phase = ToolPhase::Verifying;
                }
                persist(&opts, &mut s)?;
            }
            runtime::probe(
                &shared,
                vec![CLAUDE.into(), "--version".into()],
                &pin.version,
            )
        })();
        let mut s = state.lock().unwrap();
        if let Some(t) = s.tools.iter_mut().find(|t| t.id == "claude") {
            t.measured(result);
        }
        let _ = persist(&opts, &mut s);
    });
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsafe_members_rejected() {
        for path in [
            "/bin/codex",
            "../codex",
            "codex/../codex",
            "codex//codex",
            "codex/./codex",
            "codex\\codex",
            "",
        ] {
            assert!(member(path).is_err(), "{path}");
        }
        assert!(member("codex/bin/codex").is_ok());
    }
    #[test]
    fn catalog_and_extraction_enforce_complete_inventory() {
        let temp = tempfile::tempdir().unwrap();
        let mut catalog = fixture(&temp.path().join("source"));
        let archive = temp.path().join("source/tools.zip");
        let destination = temp.path().join("out");
        validate_catalog(&catalog).unwrap();
        extract(&archive, &catalog, &destination).unwrap();
        assert_eq!(
            fs::read(destination.join("codex/bin/codex")).unwrap(),
            b"test"
        );
        assert_eq!(
            fs::metadata(destination.join("codex/bin/codex"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert!(destination.join("codex/bin/codex-code-mode-host").is_file());
        let mut without_host = catalog.clone();
        without_host
            .files
            .retain(|f| f.path != "codex/bin/codex-code-mode-host");
        assert!(validate_catalog(&without_host).is_err());
        catalog.files[0].sha256 = "0".repeat(64);
        assert!(extract(&archive, &catalog, &temp.path().join("bad")).is_err());
        catalog.files[0].path = "../escape".into();
        assert!(validate_catalog(&catalog).is_err());
        assert!(!temp.path().join("escape").exists());
    }
    #[test]
    fn cache_is_verified_and_existing_generations_are_untouched() {
        let temp = tempfile::tempdir().unwrap();
        fixture(&temp.path().join("source"));
        let opts = Options {
            root: temp.path().join("workspace"),
            tools: Some(temp.path().join("source")),
            ..Options::default()
        };
        let (tree, _) = prepare(&opts).unwrap();
        assert!(tree.join("codex/bin/codex").is_file());
        assert!(
            !opts
                .root
                .join(".workspace/environment/generations")
                .exists()
        );
        VERIFIED.get().unwrap().lock().unwrap().remove(&tree);
        fs::write(tree.join("codex/bin/codex"), b"corrupt").unwrap();
        assert!(prepare(&opts).is_err());
    }
    #[test]
    fn tool_status_requires_execution_not_only_installed_files() {
        let temp = tempfile::tempdir().unwrap();
        fixture(&temp.path().join("source"));
        let opts = Options {
            root: temp.path().join("workspace"),
            tools: Some(temp.path().join("source")),
            runtime: Some("/bin/true".into()),
            ..Options::default()
        };
        let mut env =
            Environment::load(opts, Arc::new(std::sync::atomic::AtomicUsize::new(0))).unwrap();
        env.state.active = Some(crate::Activation {
            generation: "test".into(),
            ..crate::Activation::default()
        });
        let shared = Arc::new(Mutex::new(env));
        let result = status(&shared).unwrap();
        assert_eq!(result.tools[0].phase, ToolPhase::Failed);
        assert_eq!(result.tools[1].phase, ToolPhase::NotInstalled);
        assert_eq!(status(&shared).unwrap().revision, result.revision);
    }
    #[test]
    fn wrong_shape_state_is_preserved_without_poisoning_registry() {
        let temp = tempfile::tempdir().unwrap();
        let opts = Options {
            root: temp.path().to_path_buf(),
            ..Options::default()
        };
        let store = Store::open(&opts.root).unwrap();
        store
            .write_bytes("environment/tools/state.json", b"[]")
            .unwrap();
        let recovered = state(&opts).unwrap();
        assert!(recovered.lock().unwrap().tools.is_empty());
        let originals = store.list("corrupt").unwrap();
        assert_eq!(originals.len(), 1);
        assert_eq!(
            store.read_text(&originals[0]).unwrap().as_deref(),
            Some("[]")
        );
        assert!(state(&opts).is_ok());
    }
    #[test]
    fn invalid_archive_is_not_published() {
        let temp = tempfile::tempdir().unwrap();
        let mut catalog = fixture(&temp.path().join("source"));
        let archive = temp.path().join("tools.zip");
        fs::write(&archive, b"bad").unwrap();
        catalog.size = 3;
        catalog.sha256 = hash_file(&archive).unwrap();
        catalog.files.clear();
        assert!(extract(&archive, &catalog, &temp.path().join("out")).is_err());
        assert!(!temp.path().join("out").exists());
    }
}
#[cfg(test)]
pub(crate) fn fixture(directory: &Path) -> ToolCatalog {
    use std::io::Write;
    fs::create_dir_all(directory).unwrap();
    let archive = directory.join("tools.zip");
    let mut zip = zip::ZipWriter::new(File::create(&archive).unwrap());
    let mut files = vec![];
    for path in REQUIRED {
        zip.start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"test").unwrap();
        files.push(ToolFile {
            path: path.into(),
            sha256: persist::hash(b"test"),
            size: 4,
            executable: path.contains("/bin/"),
            extra: OpaqueObject::new(),
        });
    }
    zip.finish().unwrap();
    let tools = [("codex", "test", "/opt/workflow/tools/codex/bin/codex")]
        .into_iter()
        .map(|(id, version, binary)| ToolDefinition {
            id: id.into(),
            version: version.into(),
            binary: binary.into(),
            extra: OpaqueObject::new(),
        })
        .collect();
    let catalog = ToolCatalog {
        format: 1,
        architecture: architecture().into(),
        size: fs::metadata(&archive).unwrap().len(),
        sha256: hash_file(&archive).unwrap(),
        files,
        tools,
        optional: OptionalTools {
            claude: ClaudePin {
                version: "2.1.283".into(),
                url: None,
                sha256: None,
                bytes: None,
                extra: OpaqueObject::new(),
            },
            extra: OpaqueObject::new(),
        },
        extra: OpaqueObject::new(),
    };
    persist::write_json(&directory.join("tools.json"), &catalog).unwrap();
    catalog
}

pub(crate) fn schemas() -> std::collections::BTreeMap<String, schemars::Schema> {
    std::collections::BTreeMap::from([
        ("ToolCatalog".into(), schemars::schema_for!(ToolCatalog)),
        ("ToolState".into(), schemars::schema_for!(ToolState)),
        ("ToolsStatus".into(), schemars::schema_for!(ToolsStatus)),
    ])
}
