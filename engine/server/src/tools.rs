//! Engine-owned tool payload installation and measured backend availability.
use crate::{
    environment::{Environment, Options},
    process::Processes,
    protocol::{Error, Result},
    storage,
};
use serde_json::{Value as V, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Read,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
const PREFIX: &str = "/opt/workflow/tools";
const CLAUDE: &str = "/opt/workflow/tools/claude/bin/claude";
static VERIFIED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
static STATES: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<V>>>>> = OnceLock::new();
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
fn source(opts: &Options) -> Result<(PathBuf, V)> {
    if let Some(dir) = &opts.tools {
        return Ok((
            dir.join("tools.zip"),
            storage::read_json(&dir.join("tools.json"))?,
        ));
    }
    let apk = opts
        .apk
        .as_ref()
        .ok_or_else(|| err("Engine tool payload is not configured"))?;
    let mut z =
        zip::ZipArchive::new(File::open(apk)?).map_err(|_| err("invalid APK tool payload"))?;
    let metadata = {
        let mut file = z
            .by_name("assets/environment/tools/tools.json")
            .map_err(|_| err("missing tools catalog"))?;
        if file.size() > 4 * 1024 * 1024 {
            return Err(err("tools catalog too large"));
        }
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        crate::protocol::strict_json(&bytes).map_err(|_| err("invalid tools catalog"))?
    };
    let sha = metadata["sha256"]
        .as_str()
        .ok_or_else(|| err("missing payload digest"))?;
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
        if file.size() > 1024 * 1024 * 1024 || Some(file.size()) != metadata["size"].as_u64() {
            return Err(err("invalid payload size"));
        }
        let part = dir.join(format!(".{}", uuid::Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut dest = File::create(&part)?;
            std::io::copy(&mut file, &mut dest)?;
            dest.sync_all()?;
            if hash_file(&part)? != sha {
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
fn extract(archive: &Path, catalog: &V, target: &Path) -> Result<()> {
    if catalog["format"] != 1 || catalog["architecture"] != architecture() {
        return Err(err("unsupported tools format or architecture"));
    }
    if catalog["size"].as_u64() != Some(fs::metadata(archive)?.len())
        || catalog["sha256"] != hash_file(archive)?
    {
        return Err(err("payload checksum or size mismatch"));
    }
    let entries = catalog["files"]
        .as_array()
        .ok_or_else(|| err("missing tools inventory"))?;
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
        let name = item["path"]
            .as_str()
            .ok_or_else(|| err("invalid tool member"))?;
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
        if total > 2 * 1024 * 1024 * 1024 || Some(src.size()) != item["size"].as_u64() {
            return Err(err("tool member size mismatch"));
        }
        let path = target.join(relative);
        fs::create_dir_all(path.parent().unwrap())?;
        let mut dest = File::create(&path)?;
        std::io::copy(&mut src, &mut dest)?;
        dest.sync_all()?;
        if item["sha256"] != hash_file(&path)? {
            return Err(err("tool member checksum mismatch"));
        }
        fs::set_permissions(
            &path,
            fs::Permissions::from_mode(if item["executable"] == true {
                0o755
            } else {
                0o644
            }),
        )?;
    }
    for required in [
        "codex/bin/codex",
        "jre/bin/java",
        "chat/workflow-chat.jar",
        "notices/codex-LICENSE",
        "notices/jre-LICENSE",
        "notices/claude-code-LICENSE",
    ] {
        if !seen.contains(required) {
            return Err(err("incomplete required tools payload"));
        }
    }
    Ok(())
}
fn validate_catalog(catalog: &V) -> Result<()> {
    if catalog["format"] != 1 || catalog["architecture"] != architecture() {
        return Err(err("unsupported tools format or architecture"));
    }
    let files = catalog["files"]
        .as_array()
        .ok_or_else(|| err("missing tools inventory"))?;
    if files.len() > 10000 {
        return Err(err("too many tool members"));
    }
    let mut seen = HashSet::new();
    for file in files {
        let name = file["path"]
            .as_str()
            .ok_or_else(|| err("invalid tool member"))?;
        member(name)?;
        let sha = file["sha256"].as_str().unwrap_or("");
        if !seen.insert(name)
            || sha.len() != 64
            || !sha.bytes().all(|b| b.is_ascii_hexdigit())
            || !file["executable"].is_boolean()
            || file["size"].as_u64().is_none()
        {
            return Err(err("invalid tools inventory"));
        }
    }
    for path in [
        "codex/bin/codex",
        "jre/bin/java",
        "chat/workflow-chat.jar",
        "notices/codex-LICENSE",
        "notices/jre-LICENSE",
        "notices/claude-code-LICENSE",
    ] {
        if !seen.contains(path) {
            return Err(err("missing mandatory tool artifact"));
        }
    }
    let tools = catalog["tools"]
        .as_array()
        .ok_or_else(|| err("missing tools catalog"))?;
    if tools.len() != 3 {
        return Err(err("invalid mandatory tools"));
    }
    for (id, binary) in [
        ("codex", "/opt/workflow/tools/codex/bin/codex"),
        ("jre", "/opt/workflow/tools/jre/bin/java"),
        ("chat", "/opt/workflow/tools/chat/workflow-chat.jar"),
    ] {
        if !tools.iter().any(|t| {
            t["id"] == id
                && t["binary"] == binary
                && t["version"].as_str().is_some_and(|v| !v.is_empty())
        }) {
            return Err(err("invalid mandatory tool entry"));
        }
    }
    Ok(())
}
fn prepare(opts: &Options) -> Result<(PathBuf, V)> {
    let (archive, catalog) = source(opts)?;
    validate_catalog(&catalog)?;
    let digest = catalog["sha256"]
        .as_str()
        .ok_or_else(|| err("missing payload digest"))?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("invalid payload digest"));
    }
    let target = opts
        .root
        .join(".workspace/environment/tools/payloads")
        .join(digest);
    let mut verified = VERIFIED.get_or_init(Default::default).lock().unwrap();
    if !verified.contains(&target) {
        let valid = target.is_dir()
            && catalog["files"].as_array().is_some_and(|items| {
                items.iter().all(|i| {
                    i["path"]
                        .as_str()
                        .and_then(|n| member(n).ok())
                        .is_some_and(|p| {
                            let p = target.join(p);
                            fs::symlink_metadata(&p).is_ok_and(|m| {
                                m.file_type().is_file() && Some(m.len()) == i["size"].as_u64()
                            }) && hash_file(&p).is_ok_and(|h| i["sha256"] == h)
                        })
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
    for tool in catalog["tools"].as_array().unwrap() {
        let id = tool["id"].as_str().unwrap();
        if id == "chat" {
            continue;
        }
        let mut argv = vec![tool["binary"].as_str().unwrap().to_owned()];
        if id == "jre" {
            argv.extend(
                [
                    "-Xms16m",
                    "-Xmx256m",
                    "-XX:ActiveProcessorCount=2",
                    "-XX:+UseSerialGC",
                    "-XX:-UsePerfData",
                ]
                .map(str::to_owned),
            );
        }
        argv.push("--version".into());
        checks.push((argv, tool["version"].as_str().unwrap().to_owned()));
    }
    Ok(checks)
}
/// Apply the current payload for every generation without changing its filesystem or post-scripts.
pub fn bind(opts: &Options, command: &mut Command) -> Result<()> {
    if opts.tools.is_none() && opts.apk.is_none() {
        return Ok(());
    }
    let (tree, catalog) = prepare(opts)?;
    command
        .arg("--bind")
        .arg(format!("{}:{PREFIX}", tree.display()));
    let launcher = opts.root.join(".workspace/environment/launchers/codex");
    let script = include_bytes!("../guest/codex");
    if fs::read(&launcher).ok().as_deref() != Some(script) {
        storage::atomic(&launcher, script)?;
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
fn optional_dir(opts: &Options, catalog: &V) -> Result<PathBuf> {
    let version = catalog["optional"]["claude"]["version"]
        .as_str()
        .ok_or_else(|| err("Claude catalog missing"))?;
    storage::identifier(version)?;
    Ok(opts
        .root
        .join(".workspace/environment/tools/optional/claude")
        .join(version))
}
fn state(opts: &Options) -> Arc<Mutex<V>> {
    let mut states = STATES.get_or_init(Default::default).lock().unwrap();
    states
        .entry(opts.root.clone())
        .or_insert_with(|| {
            let mut state =
                storage::read_json(&opts.root.join(".workspace/environment/tools/state.json"))
                    .unwrap_or_else(|_| json!({"revision":0,"tools":[]}));
            state["measured"] = V::Null;
            if let Some(items) = state["tools"].as_array_mut() {
                for t in items {
                    if t["phase"] == "installing" || t["phase"] == "verifying" {
                        t["phase"] = json!("failed");
                        t["error"] = json!("Installation interrupted; retry explicitly");
                    }
                }
            }
            Arc::new(Mutex::new(state))
        })
        .clone()
}
fn persist(opts: &Options, state: &mut V) -> Result<()> {
    state["revision"] = json!(state["revision"].as_u64().unwrap_or(0) + 1);
    storage::write_json(
        &opts.root.join(".workspace/environment/tools/state.json"),
        state,
    )
}
fn probe(shared: &Arc<Mutex<Environment>>, argv: Vec<String>, expected: &str) -> Result<()> {
    struct Lease(Arc<std::sync::atomic::AtomicUsize>);
    impl Drop for Lease {
        fn drop(&mut self) {
            self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let (mut command, _lease) = {
        let env = shared.lock().unwrap();
        let command = env.process_command(&argv, "/home/work", &json!({}))?;
        env.running
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        (command, Lease(Arc::clone(&env.running)))
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            Ok(())
        });
    }
    let mut child = command.spawn()?;
    let out = child.stdout.take().unwrap();
    let error = child.stderr.take().unwrap();
    let read = |mut pipe: Box<dyn Read + Send>| {
        let mut bytes = vec![];
        let _ = pipe.by_ref().take(65536).read_to_end(&mut bytes);
        bytes
    };
    let stdout = std::thread::spawn(move || read(Box::new(out)));
    let stderr = std::thread::spawn(move || read(Box::new(error)));
    let deadline = Instant::now() + Duration::from_secs(30);
    let exit = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err("tool version check timed out"));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut bytes = stdout.join().unwrap_or_default();
    bytes.extend(stderr.join().unwrap_or_default());
    if !exit.success() || !String::from_utf8_lossy(&bytes).contains(expected) {
        return Err(err("tool execution/version check failed"));
    }
    Ok(())
}
pub fn status(shared: &Arc<Mutex<Environment>>) -> Result<V> {
    let (opts, active) = {
        let env = shared.lock().unwrap();
        (env.options.clone(), env.state["active"].clone())
    };
    let state = state(&opts);
    let mut state = state.lock().unwrap();
    if !active.is_object() {
        let tools: Vec<V> = [("codex", "/opt/workflow/tools/codex/bin/codex"), ("jre", "/opt/workflow/tools/jre/bin/java"), ("chat", "/opt/workflow/tools/chat/workflow-chat.jar"), ("claude", CLAUDE)].into_iter().map(|(id, binary)| json!({"id":id,"version":null,"architecture":architecture(),"binary":binary,"phase":"not_installed","progress":null,"error":null,"operationId":null})).collect();
        return Ok(json!({"revision":state["revision"],"tools":tools}));
    }
    let (_, catalog) = prepare(&opts)?;
    let identity = json!([catalog["sha256"], active["generation"]]);
    if state["measured"] != identity {
        let previous = state["tools"]
            .as_array()
            .and_then(|items| items.iter().find(|t| t["id"] == "claude"))
            .cloned();
        let mut items = vec![];
        for tool in catalog["tools"]
            .as_array()
            .ok_or_else(|| err("missing tool catalog"))?
        {
            let mut t = tool.clone();
            let binary = t["binary"]
                .as_str()
                .ok_or_else(|| err("invalid tool binary"))?;
            let result = if t["id"] == "chat" {
                Ok(())
            } else if t["id"] == "jre" {
                probe(
                    shared,
                    vec![
                        binary.into(),
                        "-Xms16m".into(),
                        "-Xmx256m".into(),
                        "-XX:ActiveProcessorCount=2".into(),
                        "-XX:+UseSerialGC".into(),
                        "-XX:-UsePerfData".into(),
                        "-version".into(),
                    ],
                    t["version"].as_str().unwrap_or(""),
                )
            } else {
                probe(
                    shared,
                    vec![binary.into(), "--version".into()],
                    t["version"].as_str().unwrap_or(""),
                )
            };
            t["architecture"] = json!(architecture());
            t["phase"] = json!(if result.is_ok() { "ready" } else { "failed" });
            t["progress"] = V::Null;
            t["operationId"] = V::Null;
            t["error"] = result.err().map(|e| json!(e.message)).unwrap_or(V::Null);
            items.push(t);
        }
        let pin = &catalog["optional"]["claude"];
        let binary = optional_dir(&opts, &catalog)?.join("bin/claude");
        let mut claude = json!({"id":"claude","version":pin["version"],"architecture":architecture(),"binary":CLAUDE,"phase":"not_installed","progress":null,"error":null,"operationId":null});
        if let Some(old) = previous.filter(|t| {
            t["version"] == pin["version"]
                && matches!(
                    t["phase"].as_str(),
                    Some("failed" | "installing" | "verifying")
                )
        }) {
            claude = old;
        } else if binary.is_file() {
            let result = if hash_file(&binary)? == pin["sha256"] {
                probe(
                    shared,
                    vec![CLAUDE.into(), "--version".into()],
                    pin["version"].as_str().unwrap_or(""),
                )
            } else {
                Err(err("Claude checksum mismatch"))
            };
            claude["phase"] = json!(if result.is_ok() { "ready" } else { "failed" });
            claude["error"] = result.err().map(|e| json!(e.message)).unwrap_or(V::Null);
        }
        items.push(claude);
        state["tools"] = json!(items);
        state["measured"] = identity;
        persist(&opts, &mut state)?;
    }
    Ok(json!({"revision":state["revision"],"tools":state["tools"]}))
}
pub fn install(
    shared: &Arc<Mutex<Environment>>,
    processes: Arc<Processes>,
    tool_id: &str,
    retry: bool,
) -> Result<V> {
    if tool_id != "claude" {
        return Err(Error::invalid("only optional Claude requires installation"));
    }
    let _ = status(shared)?;
    let opts = shared.lock().unwrap().options.clone();
    let (_, catalog) = prepare(&opts)?;
    let state = state(&opts);
    let mut locked = state.lock().unwrap();
    let tool = locked["tools"]
        .as_array_mut()
        .and_then(|items| items.iter_mut().find(|t| t["id"] == "claude"))
        .ok_or_else(|| err("tool status unavailable"))?;
    if matches!(
        tool["phase"].as_str(),
        Some("ready" | "installing" | "verifying")
    ) || tool["phase"] == "failed" && !retry
    {
        return Ok(json!({"revision":locked["revision"],"tools":locked["tools"]}));
    }
    tool["phase"] = json!("installing");
    tool["operationId"] = json!(uuid::Uuid::new_v4().to_string());
    tool["error"] = V::Null;
    persist(&opts, &mut locked)?;
    let response = json!({"revision":locked["revision"],"tools":locked["tools"]});
    drop(locked);
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        let result = (|| -> Result<()> {
            let pin = &catalog["optional"]["claude"];
            let url = pin["url"]
                .as_str()
                .ok_or_else(|| err("missing Claude URL"))?;
            let sha = pin["sha256"]
                .as_str()
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
            let bytes = pin["bytes"]
                .as_u64()
                .ok_or_else(|| err("missing Claude byte size"))?;
            let script = format!(
                "set -eu\np={PREFIX}/claude/bin/claude.download\ntrap 'rm -f \"$p\"' EXIT\ncurl -fsSL --connect-timeout 20 --max-time 240 --retry 3 --retry-max-time 240 -o \"$p\" '{url}'\ntest \"$(wc -c < \"$p\")\" -eq {bytes}\nprintf '%s  %s\\n' '{sha}' \"$p\" | sha256sum -c --quiet\nchmod 0755 \"$p\"\n\"$p\" --version\nmv -f \"$p\" {CLAUDE}\n"
            );
            let spawned=processes.spawn(&shared,&json!({"argv":["/bin/bash","-c",script],"cwd":"/home/work","env":{},"label":"Install Claude Code"}))?;
            let id = spawned["processId"].clone();
            let deadline = Instant::now() + Duration::from_secs(600);
            loop {
                let done =
                    processes.request("process.wait", &json!({"processId":id,"timeoutMs":1000}))?;
                if done["running"] == false {
                    if done["exitCode"] != 0 {
                        return Err(err("Claude download or verification failed"));
                    }
                    break;
                }
                if Instant::now() > deadline {
                    let _ =
                        processes.request("process.stop", &json!({"processId":id,"force":true}));
                    return Err(err("Claude installation timed out"));
                }
            }
            {
                let mut s = state.lock().unwrap();
                if let Some(t) = s["tools"]
                    .as_array_mut()
                    .and_then(|v| v.iter_mut().find(|t| t["id"] == "claude"))
                {
                    t["phase"] = json!("verifying");
                }
                persist(&opts, &mut s)?;
            }
            probe(
                &shared,
                vec![CLAUDE.into(), "--version".into()],
                pin["version"].as_str().unwrap_or(""),
            )
        })();
        let mut s = state.lock().unwrap();
        if let Some(t) = s["tools"]
            .as_array_mut()
            .and_then(|v| v.iter_mut().find(|t| t["id"] == "claude"))
        {
            t["phase"] = json!(if result.is_ok() { "ready" } else { "failed" });
            t["error"] = result.err().map(|e| json!(e.message)).unwrap_or(V::Null);
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
            "/bin/java",
            "../java",
            "jre/../java",
            "jre//java",
            "jre/./java",
            "jre\\java",
            "",
        ] {
            assert!(member(path).is_err(), "{path}")
        }
        assert!(member("jre/bin/java").is_ok());
    }
    #[test]
    fn catalog_and_extraction_enforce_complete_inventory() {
        let temp = tempfile::tempdir().unwrap();
        let mut catalog = fixture(&temp.path().join("source"));
        let archive = temp.path().join("source/tools.zip");
        let destination = temp.path().join("out");
        validate_catalog(&catalog).unwrap();
        extract(&archive, &catalog, &destination).unwrap();
        assert_eq!(fs::read(destination.join("jre/bin/java")).unwrap(), b"test");
        assert_eq!(
            fs::metadata(destination.join("jre/bin/java"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        catalog["files"][0]["sha256"] = json!("0".repeat(64));
        assert!(extract(&archive, &catalog, &temp.path().join("bad")).is_err());
        catalog["files"][0]["path"] = json!("../escape");
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
        env.state["active"] = json!({"generation":"test","environment":{}});
        let shared = Arc::new(Mutex::new(env));
        let result = status(&shared).unwrap();
        assert_eq!(result["tools"][0]["phase"], "failed");
        assert_eq!(result["tools"][1]["phase"], "failed");
        assert_eq!(result["tools"][3]["phase"], "not_installed");
        assert_eq!(status(&shared).unwrap()["revision"], result["revision"]);
    }
    #[test]
    fn invalid_archive_is_not_published() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("tools.zip");
        fs::write(&archive, b"bad").unwrap();
        assert!(extract(&archive,&json!({"format":1,"architecture":architecture(),"size":3,"sha256":hash_file(&archive).unwrap(),"files":[]}),&temp.path().join("out")).is_err());
        assert!(!temp.path().join("out").exists());
    }
}

#[cfg(test)]
pub(crate) fn fixture(directory: &Path) -> V {
    use std::io::Write;
    fs::create_dir_all(directory).unwrap();
    let archive = directory.join("tools.zip");
    let mut z = zip::ZipWriter::new(File::create(&archive).unwrap());
    let mut files = vec![];
    for path in [
        "codex/bin/codex",
        "jre/bin/java",
        "chat/workflow-chat.jar",
        "notices/codex-LICENSE",
        "notices/jre-LICENSE",
        "notices/claude-code-LICENSE",
    ] {
        z.start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(b"test").unwrap();
        files.push(json!({"path":path,"sha256":storage::hash(b"test"),"size":4,"executable":path.contains("/bin/")}));
    }
    z.finish().unwrap();
    let catalog = json!({"format":1,"architecture":architecture(),"size":fs::metadata(&archive).unwrap().len(),"sha256":hash_file(&archive).unwrap(),"files":files,"tools":[{"id":"codex","version":"test","binary":"/opt/workflow/tools/codex/bin/codex"},{"id":"jre","version":"test","binary":"/opt/workflow/tools/jre/bin/java"},{"id":"chat","version":"1.0.0","binary":"/opt/workflow/tools/chat/workflow-chat.jar"}],"optional":{"claude":{"version":"2.1.283"}}});
    storage::write_json(&directory.join("tools.json"), &catalog).unwrap();
    catalog
}
