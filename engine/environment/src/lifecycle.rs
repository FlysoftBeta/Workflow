use crate::{
    Error, Result, Store, access,
    json::{OpaqueJson, OpaqueObject, canonical_bytes},
    model::*,
    persist::{self, now},
    runtime,
    store::keys::{CONFIG, ENVIRONMENT_STATE, GENERATIONS, IMAGES, PERSISTENT_STORES, TOOLCHAINS},
};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
const ENVCTL: &str = "/usr/local/libexec/workflow/envctl";
/// A host path below `.workspace/`.
fn internal(opts: &Options, key: &str) -> PathBuf {
    opts.root.join(access::DIRECTORY).join(key)
}
fn generation_dir(opts: &Options, id: &str) -> Result<PathBuf> {
    Ok(internal(opts, GENERATIONS).join(persist::identifier(id)?))
}
/// The host directory of an image store. The toolchains are reinstalled by every build, so they
/// live in the cache; other stores, such as the guest home, hold user data.
pub(crate) fn store_dir(opts: &Options, name: &str) -> PathBuf {
    if name == "toolchains" {
        internal(opts, TOOLCHAINS)
    } else {
        internal(opts, PERSISTENT_STORES).join(name)
    }
}
/// The `environment` section of `config.json`, the only part of that file Environment reads.
#[derive(serde::Deserialize)]
struct Declaration<T> {
    environment: Option<T>,
}
#[derive(Clone, Default)]
pub struct Options {
    pub root: PathBuf,
    pub runtime: Option<PathBuf>,
    pub loader: Option<PathBuf>,
    pub apk: Option<PathBuf>,
    pub tools: Option<PathBuf>,
    pub image: Option<PathBuf>,
    pub image_index: Option<PathBuf>,
}
pub struct Environment {
    pub options: Options,
    pub state: EnvironmentState,
    pub building: bool,
    read_only: bool,
    store: Store,
    pub running: Arc<AtomicUsize>,
}
impl Environment {
    pub fn load(options: Options, running: Arc<AtomicUsize>) -> Result<Self> {
        let store = Store::open(&options.root)?;
        let generations = store.path(GENERATIONS)?;
        fs::create_dir_all(&generations)?;
        fs::create_dir_all(store.path(PERSISTENT_STORES)?)?;
        let mut read_only = false;
        #[derive(serde::Deserialize)]
        struct Format {
            format: u64,
        }
        let mut state = match store.read::<Format>(ENVIRONMENT_STATE) {
            Ok(None) => EnvironmentState::default(),
            Ok(Some(header)) if header.format != 1 => {
                read_only = true;
                EnvironmentState {
                    status: LifecycleStatus::Failed,
                    failure: Some(Failure::new(
                        "recovery",
                        "Unsupported environment state; preserved read-only",
                        None,
                    )),
                    ..EnvironmentState::default()
                }
            }
            _ => match store.read::<EnvironmentState>(ENVIRONMENT_STATE) {
                Ok(Some(state)) => state,
                _ => {
                    store.quarantine(ENVIRONMENT_STATE)?;
                    let mut restored = store
                        .read::<EnvironmentState>(&format!("{ENVIRONMENT_STATE}.bak"))
                        .ok()
                        .flatten()
                        .filter(|v| v.format == 1)
                        .unwrap_or_default();
                    restored.status = LifecycleStatus::Failed;
                    restored.failure = Some(Failure::new(
                        "recovery",
                        "Damaged environment state preserved; last valid state restored when available",
                        None,
                    ));
                    restored
                }
            },
        };
        if state.status == LifecycleStatus::Applying {
            if let Some(job) = state
                .job_id
                .as_ref()
                .filter(|id| persist::identifier(id).is_ok())
            {
                crate::home_stage::cleanup(&generations.join(job));
            }
            state.status = if state.active.is_some() {
                LifecycleStatus::Ready
            } else {
                LifecycleStatus::Unavailable
            };
            state.failure = Some(Failure::new(
                "interrupted",
                "Previous build was interrupted; retry to rebuild",
                None,
            ));
        }
        if !read_only {
            forget_missing_generations(&mut state, &generations, &store_dir(&options, "toolchains"));
        }
        let out = Self {
            options,
            state,
            building: false,
            read_only,
            store,
            running,
        };
        if !out.read_only {
            out.persist()?;
        }
        Ok(out)
    }
    /// Identifies the requested declaration: only the `environment` section of `config.json`
    /// counts, so editing other settings never schedules a build.
    fn requested_spec_hash(&self) -> String {
        match self.store.read::<Declaration<OpaqueJson>>(CONFIG) {
            Ok(Some(declaration)) => persist::hash(&canonical_bytes(&declaration.environment)),
            Ok(None) => "missing".into(),
            Err(error) => format!("unreadable:{}", error.kind),
        }
    }
    pub fn reconcile_changed(shared: &Arc<Mutex<Self>>) {
        let changed = {
            let e = shared.lock().unwrap();
            // A removed cache leaves an enrolled environment without its generation; rebuild it
            // like a changed declaration. Failed builds keep their fingerprint guard.
            let evicted = e.state.active.is_none()
                && e.state.failure.is_none()
                && e.state.status == LifecycleStatus::Unavailable;
            !e.read_only
                && !e.building
                && e.options.runtime.is_some()
                && (e.options.apk.is_some() || e.options.image.is_some())
                && (e.state.requested_spec_hash.is_some() || e.state.active.is_some())
                && (evicted
                    || e.state.requested_spec_hash.as_deref()
                        != Some(e.requested_spec_hash().as_str()))
        };
        if changed {
            let _ = Self::reconcile(shared, false);
        }
    }
    fn persist(&self) -> Result<()> {
        if self.read_only {
            return Err(Error::business(
                "unsupported_format",
                "environment state is read-only",
            ));
        }
        self.store.backup(ENVIRONMENT_STATE)?;
        self.store.write(ENVIRONMENT_STATE, &self.state)
    }
    pub fn status(&self) -> EnvironmentStatus {
        EnvironmentStatus {
            state: self.state.clone(),
            running_processes: self.running.load(Ordering::SeqCst),
            environment_available: self.state.active.is_some(),
            usable: self.state.active.is_some(),
            phase: match self.state.status {
                LifecycleStatus::Ready => EnvironmentPhase::Ready,
                LifecycleStatus::PendingRestart => EnvironmentPhase::NeedsRestart,
                LifecycleStatus::Failed => EnvironmentPhase::Failed,
                LifecycleStatus::Applying
                    if matches!(self.state.stage.as_str(), "install" | "image") =>
                {
                    EnvironmentPhase::Installing
                }
                LifecycleStatus::Applying => EnvironmentPhase::Building,
                _ => EnvironmentPhase::NotInstalled,
            },
            progress: (self.state.steps > 0)
                .then(|| self.state.step as f64 / self.state.steps as f64),
            error: self.state.failure.as_ref().map(|f| f.message.clone()),
            architecture: architecture().into(),
        }
    }
    pub fn reconcile(shared: &Arc<Mutex<Self>>, retry: bool) -> Result<EnvironmentStatus> {
        let (mut spec, opts, active, job, running);
        {
            let mut e = shared.lock().unwrap();
            if e.read_only {
                return Err(Error::business(
                    "unsupported_format",
                    "environment state is read-only",
                ));
            }
            if e.building {
                return Ok(e.status());
            }
            e.state.requested_spec_hash = Some(e.requested_spec_hash());
            spec = match read_spec(&e.store) {
                Ok(v) => v,
                Err(error) => {
                    e.state.status = LifecycleStatus::Failed;
                    e.state.stage = "config".into();
                    e.state.failure = Some(Failure::new("config", error.message, None));
                    e.persist()?;
                    return Ok(e.status());
                }
            };
            opts = e.options.clone();
            active = e.state.active.clone();
            job = uuid::Uuid::new_v4().to_string();
            running = e.running.clone();
            let fingerprint = persist::hash(&canonical_bytes(&spec));
            if !retry
                && e.state
                    .failure
                    .as_ref()
                    .and_then(|f| f.fingerprint.as_ref())
                    == Some(&fingerprint)
            {
                return Ok(e.status());
            }
            e.state.status = LifecycleStatus::Applying;
            e.state.stage = "image".into();
            e.state.job_id = Some(job.clone());
            e.state.step = 0;
            e.state.steps = 0;
            if let Err(error) = e.persist() {
                e.state.status = LifecycleStatus::Failed;
                e.state.failure = Some(Failure::new("persist", error.message.clone(), None));
                return Err(error);
            }
            e.building = true;
        }
        let shared2 = shared.clone();
        std::thread::spawn(move || {
            let fingerprint = persist::hash(&canonical_bytes(&spec));
            let outcome = build(&opts, &mut spec, active.as_ref(), &job, &shared2);
            let generation = internal(&opts, GENERATIONS).join(&job);
            if outcome.is_err() {
                crate::home_stage::cleanup(&generation);
            }
            let current_fingerprint = Store::open(&opts.root)
                .and_then(|store| read_spec(&store))
                .ok()
                .map(|v| persist::hash(&canonical_bytes(&v)));
            let mut e = shared2.lock().unwrap();
            if current_fingerprint.as_deref() != Some(&fingerprint) {
                e.building = false;
                e.state.status = if e.state.active.is_some() {
                    LifecycleStatus::Ready
                } else {
                    LifecycleStatus::Unavailable
                };
                e.state.stage = "configuration_changed".into();
                let _ = e.persist();
                crate::home_stage::cleanup(&generation);
                drop(e);
                let _ = Self::reconcile(&shared2, false);
                return;
            }
            e.building = false;
            match outcome {
                Ok(activation) => {
                    e.state.failure = None;
                    if e.state.active.as_ref() == Some(&activation) {
                        e.state.pending = None;
                        e.state.status = LifecycleStatus::Ready;
                    } else if running.load(Ordering::SeqCst) > 0 {
                        e.state.pending = Some(activation);
                        e.state.status = LifecycleStatus::PendingRestart;
                    } else {
                        match activate(&opts, &activation) {
                            Ok(()) => {
                                e.state.previous = e.state.active.take();
                                e.state.active = Some(activation);
                                e.state.pending = None;
                                e.state.status = LifecycleStatus::Ready;
                            }
                            Err(err) => {
                                crate::home_stage::cleanup(&generation);
                                e.state.status = LifecycleStatus::Failed;
                                e.state.failure =
                                    Some(Failure::new("activate", err.message, Some(fingerprint)));
                            }
                        }
                    }
                }
                Err(err) => {
                    e.state.status = LifecycleStatus::Failed;
                    e.state.failure =
                        Some(Failure::new(&e.state.stage, err.message, Some(fingerprint)));
                }
            }
            if let Err(err) = e.persist() {
                e.state.status = LifecycleStatus::Failed;
                e.state.failure = Some(Failure::new("persist", err.message, None));
            }
        });
        Ok(shared.lock().unwrap().status())
    }
    pub fn has_verified_pending(&self) -> Result<bool> {
        if self.read_only {
            return Err(Error::business(
                "unsupported_format",
                "environment state is read-only",
            ));
        }
        if self.building {
            return Err(Error::business(
                "busy",
                "environment build is still running",
            ));
        }
        let Some(pending) = self.state.pending.as_ref() else {
            return Ok(false);
        };
        verify_generation(&self.options, pending)?;
        Ok(true)
    }
    pub fn restart(&mut self) -> Result<EnvironmentStatus> {
        if self.read_only {
            return Err(Error::business(
                "unsupported_format",
                "environment state is read-only",
            ));
        }
        if self.building {
            return Err(Error::business(
                "busy",
                "environment build is still running",
            ));
        }
        if self.running.load(Ordering::SeqCst) > 0 {
            return Err(Error::business(
                "processes_running",
                "stop environment processes before restarting",
            ));
        }
        let activation = self
            .state
            .pending
            .clone()
            .or_else(|| self.state.active.clone())
            .ok_or_else(|| Error::business("unavailable", "no verified environment"))?;
        verify_generation(&self.options, &activation)?;
        if let Err(error) = activate(&self.options, &activation) {
            if let Ok(generation) = generation_dir(&self.options, &activation.generation) {
                crate::home_stage::cleanup(&generation);
            }
            self.state.pending = None;
            self.state.status = LifecycleStatus::Failed;
            self.state.failure = Some(Failure::new("activate", error.message.clone(), None));
            self.persist()?;
            return Err(error);
        }
        if self.state.active.as_ref() != Some(&activation) {
            self.state.previous = self.state.active.take();
        }
        self.state.active = Some(activation);
        self.state.pending = None;
        self.state.status = LifecycleStatus::Ready;
        self.persist()?;
        Ok(self.status())
    }
    pub(crate) fn process_command(
        &self,
        argv: &[String],
        cwd: &str,
        env: &BTreeMap<String, String>,
    ) -> Result<std::process::Command> {
        let active = self.state.active.as_ref().ok_or_else(|| {
            Error::business(
                "environment_unavailable",
                "no verified environment has been activated",
            )
        })?;
        runtime::validate_command(argv, cwd, env)?;
        let generation = generation_dir(&self.options, &active.generation)?;
        let mut command = runtime::guest_command(
            &self.options,
            &generation,
            "work",
            cwd,
            &active.environment,
            true,
        )?;
        for (key, value) in env {
            command.arg(format!("{key}={value}"));
        }
        command.args(argv);
        Ok(command)
    }
}
fn read_spec(store: &Store) -> Result<EnvironmentSpec> {
    let declaration: Declaration<EnvironmentSpec> = store
        .read(CONFIG)?
        .ok_or_else(|| Error::business("io", "configuration not found"))?;
    // A configuration without the section declares the image defaults.
    let spec = declaration.environment.unwrap_or_default();
    validate(&spec)?;
    Ok(spec)
}
/// Forget activations whose generation or toolchains were removed with the cache. The
/// environment becomes unavailable and is rebuilt from its declaration.
fn forget_missing_generations(state: &mut EnvironmentState, generations: &Path, toolchains: &Path) {
    let missing = |activation: &Activation| {
        persist::identifier(&activation.generation).is_err()
            || !generations.join(&activation.generation).is_dir()
            || !toolchains.is_dir()
    };
    if state.previous.as_ref().is_some_and(missing) {
        state.previous = None;
    }
    if state.pending.as_ref().is_some_and(missing) {
        state.pending = None;
        if state.status == LifecycleStatus::PendingRestart {
            state.status = LifecycleStatus::Ready;
        }
    }
    if state.active.as_ref().is_some_and(missing) {
        state.active = None;
        state.status = LifecycleStatus::Unavailable;
        state.stage = "cache_removed".into();
        state.failure = None;
    }
}
pub fn validate(spec: &EnvironmentSpec) -> Result<()> {
    if spec.version != 1 {
        return Err(Error::invalid("environment.version must be 1"));
    }
    for (lang, list) in [("python", &spec.python), ("node", &spec.node)] {
        let mut seen = HashSet::new();
        for version in list.iter().flatten() {
            let parts: Vec<_> = version.split('.').collect();
            if parts.is_empty()
                || parts.len() > 3
                || parts.iter().any(|p| {
                    p.is_empty()
                        || !p.bytes().all(|b| b.is_ascii_digit())
                        || (p.len() > 1 && p.starts_with('0'))
                        || p.parse::<u16>().is_err()
                })
                || (lang == "python" && parts[0] != "3")
                || (lang == "node" && parts[0].parse::<u16>().unwrap_or(0) < 1)
                || !seen.insert(version)
            {
                return Err(Error::invalid(&format!(
                    "environment.{lang}: duplicate or invalid version"
                )));
            }
        }
    }
    let mut seen = HashSet::new();
    for package in spec.packages.iter().flatten() {
        if package.is_empty()
            || !package.bytes().all(|b| {
                b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || matches!(b, b'+' | b'-' | b'.' | b':')
            })
            || !package.as_bytes()[0].is_ascii_alphanumeric()
            || !seen.insert(package)
        {
            return Err(Error::invalid("environment.packages: invalid or duplicate package"));
        }
    }
    for (key, value) in spec.env.iter().flatten() {
        if !runtime::env_key(key) || value.contains('\0') {
            return Err(Error::invalid("environment.env: invalid variable"));
        }
    }
    let mut seen = HashSet::new();
    for script in spec.post_scripts.iter().flatten() {
        persist::identifier(&script.id)?;
        if !seen.insert(&script.id) || script.run.trim().is_empty() || script.run.contains('\0') {
            return Err(Error::invalid(
                "environment.post_scripts: duplicate id or invalid script",
            ));
        }
    }
    Ok(())
}
pub(crate) fn architecture() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "amd64"
    }
}
fn progress(shared: &Arc<Mutex<Environment>>, stage: &str, step: usize, steps: usize) {
    let mut e = shared.lock().unwrap();
    e.state.stage = stage.into();
    e.state.step = step;
    e.state.steps = steps;
    let _ = e.persist();
}
fn image(opts: &Options) -> Result<(PathBuf, PathBuf, ImageIndex)> {
    let (image, index) = if let (Some(image), Some(index)) = (&opts.image, &opts.image_index) {
        (image.clone(), index.clone())
    } else if let Some(apk) = &opts.apk {
        let dir = internal(opts, IMAGES);
        fs::create_dir_all(&dir)?;
        let mut zip = zip::ZipArchive::new(File::open(apk)?)
            .map_err(|_| Error::business("invalid_apk", "APK cannot be read"))?;
        let index_bytes = {
            let mut asset = zip
                .by_name("assets/environment/image.json")
                .map_err(|_| Error::business("missing_image", "APK index is missing"))?;
            if asset.size() > 16 * 1024 * 1024 {
                return Err(Error::business("too_large", "image index exceeds limit"));
            }
            let mut bytes = vec![];
            asset.read_to_end(&mut bytes)?;
            bytes
        };
        let metadata: ImageIndex = crate::json::strict_json(&index_bytes)
            .map_err(|_| Error::business("invalid_image", "invalid image index"))?;
        let sha = &metadata.sha256;
        if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::business("invalid_image", "invalid image checksum"));
        }
        let image = dir.join(format!("{sha}.tar.zst"));
        let index = dir.join(format!("{sha}.json"));
        if !image.exists() {
            let mut asset = zip
                .by_name("assets/environment/image.tar.zst")
                .map_err(|_| Error::business("missing_image", "APK customized image is missing"))?;
            if asset.size() > 8u64 * 1024 * 1024 * 1024 {
                return Err(Error::business("too_large", "image exceeds limit"));
            }
            let temp = dir.join(format!(".extract-{}", uuid::Uuid::new_v4()));
            let outcome = (|| -> Result<()> {
                let mut file = File::create(&temp)?;
                std::io::copy(&mut asset, &mut file)?;
                file.sync_all()?;
                fs::rename(&temp, &image)?;
                Ok(())
            })();
            if outcome.is_err() {
                let _ = fs::remove_file(&temp);
            }
            outcome?;
        }
        persist::atomic(&index, &index_bytes)?;
        (image, index)
    } else {
        return Err(Error::business(
            "image_unavailable",
            "customized image and index or APK are required",
        ));
    };
    let metadata: ImageIndex = persist::read_json(&index)?;
    let m = &metadata.metadata;
    if m.format != "workflow-image"
        || m.format_version != 2
        || m.profile != "workspace"
        || m.image_type != "debian-trixie"
        || m.type_version != 1
    {
        return Err(Error::business(
            "unsupported_image",
            "a customized workflow workspace image is required",
        ));
    }
    Ok((image, index, metadata))
}
fn merge_seed(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    let m = fs::symlink_metadata(from)?;
    if m.file_type().is_symlink() {
        if !to.exists() && fs::symlink_metadata(to).is_err() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(from)?, to)?;
        }
        return Ok(());
    }
    if m.is_dir() {
        fs::create_dir_all(to)?;
        for e in fs::read_dir(from)? {
            let e = e?;
            merge_seed(&e.path(), &to.join(e.file_name()))?;
        }
    } else if !to.exists() {
        fs::copy(from, to)?;
    }
    Ok(())
}
fn build(
    opts: &Options,
    spec: &mut EnvironmentSpec,
    active: Option<&Activation>,
    job: &str,
    shared: &Arc<Mutex<Environment>>,
) -> Result<Activation> {
    let (image, index, metadata) = image(opts)?;
    spec.python
        .get_or_insert_with(|| metadata.metadata.defaults.python.versions());
    spec.node
        .get_or_insert_with(|| metadata.metadata.defaults.node.versions());
    spec.packages.get_or_insert_with(Vec::new);
    spec.env.get_or_insert_with(BTreeMap::new);
    spec.post_scripts.get_or_insert_with(Vec::new);
    validate(spec)?;
    let image_sha = &metadata.sha256;
    let fingerprint = persist::hash(&canonical_bytes(spec));
    if let Some(active) =
        active.filter(|a| a.fingerprint == fingerprint && a.image_sha256 == *image_sha)
    {
        verify_generation(opts, active)?;
        return Ok(active.clone());
    }
    let generation = generation_dir(opts, job)?;
    let python = spec.python.as_ref().unwrap();
    let node = spec.node.as_ref().unwrap();
    let scripts = spec.post_scripts.as_ref().unwrap();
    let total = 4 + python.len() + node.len() + scripts.len();
    let mut step = 1;
    progress(shared, "install", step, total);
    runtime::install_generation(opts, &image, &index, &generation)?;
    for seed in metadata.metadata.stores.values() {
        let name = &seed.store;
        if name.starts_with('/')
            || name
                .split('/')
                .any(|s| s.is_empty() || s == ".." || s == ".")
        {
            return Err(Error::business("invalid_image", "invalid seed store path"));
        }
        let target = store_dir(opts, name);
        if seed.seed != "if-absent" || !target.exists() {
            merge_seed(&generation.join("seeds").join(name), &target)?;
        }
    }
    let mut merged_env = metadata.metadata.environment.clone();
    merged_env.extend(spec.env.as_ref().unwrap().clone());
    let process_env = merged_env.clone();
    let packages = spec.packages.as_ref().unwrap();
    step += 1;
    progress(shared, "packages", step, total);
    if !packages.is_empty() {
        let mut cmd = runtime::guest_command(opts, &generation, "root", "/", &merged_env, false)?;
        cmd.arg(ENVCTL).arg("apt-install").args(packages);
        runtime::checked(cmd, "package install")?;
    }
    for (lang, versions) in [("python", python), ("node", node)] {
        for version in versions {
            step += 1;
            progress(shared, lang, step, total);
            let mut cmd = runtime::guest_command(
                opts,
                &generation,
                "work",
                "/home/work",
                &merged_env,
                false,
            )?;
            cmd.arg(ENVCTL).arg(lang).arg(version);
            runtime::checked(cmd, &format!("{lang} install"))?;
        }
    }
    step += 1;
    progress(shared, "verify", step, total);
    let mut cmd =
        runtime::guest_command(opts, &generation, "work", "/home/work", &merged_env, false)?;
    cmd.arg(ENVCTL)
        .arg("verify-many")
        .arg(String::from_utf8(canonical_bytes(python)).unwrap())
        .arg(String::from_utf8(canonical_bytes(node)).unwrap())
        .args(packages);
    let bytes = runtime::checked(cmd, "toolchain verification")?;
    let verified: Verification = crate::json::strict_json(&bytes).map_err(|_| {
        Error::business(
            "verification_failed",
            "invalid toolchain verification response",
        )
    })?;
    persist::identifier(&verified.profile)?;
    for (expected, actual) in [(python, &verified.python), (node, &verified.node)] {
        if expected.len() != actual.len()
            || !expected
                .iter()
                .zip(actual)
                .all(|(want, got)| got == want || got.starts_with(&format!("{want}.")))
        {
            return Err(Error::business(
                "verification_failed",
                "installed language versions do not match configuration",
            ));
        }
    }
    if packages.iter().any(|p| !verified.packages.contains_key(p)) {
        return Err(Error::business(
            "verification_failed",
            "required packages missing",
        ));
    }
    verify_tools(opts, &generation, &merged_env)?;
    let profile = &verified.profile;
    merged_env.insert("PATH".into(), format!("/opt/toolchains/profiles/{profile}/python/bin:/opt/toolchains/profiles/{profile}/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"));
    if !scripts.is_empty() {
        crate::home_stage::prepare(&opts.root, &generation)?;
    }
    for script in scripts {
        step += 1;
        progress(shared, &format!("post_script:{}", script.id), step, total);
        let mut cmd = runtime::guest_command(
            opts,
            &generation,
            script.user.as_str(),
            "/",
            &merged_env,
            false,
        )?;
        cmd.arg("/bin/bash").arg("-euc").arg(&script.run);
        runtime::checked(cmd, &format!("post script {}", script.id))?;
    }
    let activation = Activation {
        generation: job.into(),
        profile: profile.clone(),
        config: spec.clone(),
        environment: process_env,
        fingerprint,
        image_sha256: image_sha.clone(),
        verified,
        verified_at: now(),
        extra: OpaqueObject::new(),
    };
    persist::write_json(&generation.join("workspace-activation.json"), &activation)?;
    step += 1;
    progress(shared, "activate", step, total);
    verify_generation(opts, &activation)?;
    Ok(activation)
}
fn verify_tools(opts: &Options, generation: &Path, env: &BTreeMap<String, String>) -> Result<()> {
    for (argv, version) in crate::tools::verification_commands(opts)? {
        let mut command =
            runtime::guest_command(opts, generation, "work", "/home/work", env, false)?;
        command.args(&argv);
        let output = runtime::checked(command, "required tool verification")?;
        if !String::from_utf8_lossy(&output).contains(&version) {
            return Err(Error::business(
                "tool_version_mismatch",
                "Required Engine tool version does not match its pin",
            ));
        }
    }
    Ok(())
}
fn verify_generation(opts: &Options, activation: &Activation) -> Result<()> {
    let generation = generation_dir(opts, &activation.generation)?;
    runtime::verify_generation(opts, &generation)?;
    verify_tools(opts, &generation, &activation.environment)
}
fn activate(opts: &Options, activation: &Activation) -> Result<()> {
    let generation = generation_dir(opts, &activation.generation)?;
    let mut cmd = runtime::guest_command(
        opts,
        &generation,
        "work",
        "/home/work",
        activation.config.env.as_ref().unwrap_or(&BTreeMap::new()),
        false,
    )?;
    cmd.arg(ENVCTL).arg("activate").arg(&activation.profile);
    let active_link = store_dir(opts, "toolchains").join("active");
    let previous = fs::read_link(&active_link).ok();
    runtime::checked(cmd, "profile activation")?;
    if let Err(error) = crate::home_stage::activate(&opts.root, &generation) {
        if let Some(previous) = previous {
            let temp = active_link.with_file_name(format!(".restore-{}", uuid::Uuid::new_v4()));
            std::os::unix::fs::symlink(previous, &temp)?;
            fs::rename(temp, &active_link)?;
        } else {
            let _ = fs::remove_file(&active_link);
        }
        return Err(error);
    }
    crate::home_stage::cleanup(&generation);
    Ok(())
}

impl Options {
    /// Canonicalize bootstrap inputs before any lifecycle state is opened.
    pub fn prepare(mut self) -> Result<Self> {
        if self.root.as_os_str().is_empty() {
            return Err(Error::invalid("--root is required"));
        }
        fs::create_dir_all(&self.root)?;
        self.root = self.root.canonicalize()?;
        for path in [
            &mut self.runtime,
            &mut self.loader,
            &mut self.apk,
            &mut self.tools,
            &mut self.image,
            &mut self.image_index,
        ]
        .into_iter()
        .flatten()
        {
            *path = path.canonicalize()?;
        }
        if self.image.is_some() != self.image_index.is_some() {
            return Err(Error::invalid(
                "--image and --image-index must be provided together",
            ));
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn environment(root: &Path) -> Environment {
        Environment::load(
            Options {
                root: root.into(),
                ..Options::default()
            },
            Arc::new(AtomicUsize::new(0)),
        )
        .unwrap()
    }
    #[test]
    fn declaration_fingerprint_preserves_missing_fields_and_canonical_key_order() {
        let original = br#"{"env":{"A":"first","Z":"last"},"node":[],"post_scripts":[{"id":"setup","run":"true","user":"work"}],"python":["3.13"],"version":1}"#;
        let spec: EnvironmentSpec = crate::json::strict_json(original).unwrap();
        validate(&spec).unwrap();
        assert_eq!(canonical_bytes(&spec), original);
        let missing: EnvironmentSpec = crate::json::strict_json(br#"{"version":1}"#).unwrap();
        assert_eq!(canonical_bytes(&missing), br#"{"version":1}"#);
        assert!(missing.python.is_none());
        let empty: EnvironmentSpec =
            crate::json::strict_json(br#"{"python":[],"version":1}"#).unwrap();
        assert_eq!(empty.python, Some(vec![]));
    }
    #[test]
    fn malformed_typed_declarations_reject_wrong_shapes_and_duplicates() {
        for raw in [
            r#"{"version":1,"python":"3.13"}"#,
            r#"{"version":1,"python":null}"#,
            r#"{"version":1,"env":{"X":false}}"#,
            r#"{"version":1,"post_scripts":[{"id":"x","run":"true","user":"host"}]}"#,
            r#"{"version":1,"extra":{"x":1,"x":2}}"#,
        ] {
            assert!(
                crate::json::strict_json::<EnvironmentSpec>(raw.as_bytes()).is_err(),
                "{raw}"
            );
        }
        let duplicate: EnvironmentSpec =
            crate::json::strict_json(br#"{"version":1,"python":["3.13","3.13"]}"#).unwrap();
        assert!(validate(&duplicate).is_err());
    }
    #[test]
    fn corrupt_state_is_quarantined_and_unknown_format_is_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .write(
                "environment/environment.json.bak",
                &EnvironmentState::default(),
            )
            .unwrap();
        store
            .write_bytes("environment/environment.json", b"broken")
            .unwrap();
        let recovered = environment(dir.path());
        assert_eq!(recovered.state.failure.as_ref().unwrap().stage, "recovery");
        assert_eq!(store.list("corrupt").unwrap().len(), 1);
        store
            .write_bytes(
                "environment/environment.json",
                br#"{"format":99,"future":{"id":1}}"#,
            )
            .unwrap();
        let future = environment(dir.path());
        assert_eq!(future.state.status, LifecycleStatus::Failed);
        assert!(future.read_only);
        assert_eq!(
            store
                .read_text("environment/environment.json")
                .unwrap()
                .as_deref(),
            Some(r#"{"format":99,"future":{"id":1}}"#)
        );
    }
    #[test]
    fn interrupted_build_removes_private_home_stage_and_retains_active_generation() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let state = EnvironmentState {
            status: LifecycleStatus::Applying,
            job_id: Some("job".into()),
            active: Some(Activation {
                generation: "previous".into(),
                ..Activation::default()
            }),
            ..EnvironmentState::default()
        };
        store.write("environment/environment.json", &state).unwrap();
        store
            .write_text("cache/generations/job/post-home/secret", "do not retain")
            .unwrap();
        fs::create_dir_all(store.path("cache/generations/previous").unwrap()).unwrap();
        fs::create_dir_all(store.path("cache/toolchains").unwrap()).unwrap();
        let recovered = environment(dir.path());
        assert_eq!(recovered.state.status, LifecycleStatus::Ready);
        assert!(recovered.status().usable);
        assert!(!store.exists("cache/generations/job/post-home").unwrap());
        assert_eq!(recovered.state.failure.unwrap().stage, "interrupted");
    }
    #[test]
    fn removed_cache_forgets_generations_and_requests_a_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let activation = |generation: &str| Activation {
            generation: generation.into(),
            ..Activation::default()
        };
        let state = EnvironmentState {
            status: LifecycleStatus::PendingRestart,
            active: Some(activation("active")),
            pending: Some(activation("pending")),
            previous: Some(activation("previous")),
            requested_spec_hash: Some("hash".into()),
            ..EnvironmentState::default()
        };
        store.write("environment/environment.json", &state).unwrap();
        for kept in ["cache/generations/active", "cache/toolchains"] {
            fs::create_dir_all(store.path(kept).unwrap()).unwrap();
        }
        let partial = environment(dir.path());
        assert_eq!(partial.state.active, Some(activation("active")));
        assert_eq!(partial.state.pending, None);
        assert_eq!(partial.state.previous, None);
        assert_eq!(partial.state.status, LifecycleStatus::Ready);
        fs::remove_dir_all(store.path("cache").unwrap()).unwrap();
        let evicted = environment(dir.path());
        assert_eq!(evicted.state.active, None);
        assert_eq!(evicted.state.status, LifecycleStatus::Unavailable);
        assert_eq!(evicted.state.stage, "cache_removed");
        assert!(evicted.state.failure.is_none());
        assert!(store.path("cache/generations").unwrap().is_dir());
    }
    #[test]
    fn declaration_is_the_environment_section_of_the_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let env = environment(dir.path());
        assert_eq!(env.requested_spec_hash(), "missing");
        assert_eq!(read_spec(&store).unwrap_err().kind, "io");
        store
            .write_text("config.json", r#"{"version":2,"appearance":{"theme":"dark"}}"#)
            .unwrap();
        let defaults = env.requested_spec_hash();
        assert_eq!(read_spec(&store).unwrap(), EnvironmentSpec::default());
        store
            .write_text(
                "config.json",
                r#"{"version":2,"appearance":{"theme":"light"},"environment":{"version":1,"python":["3.13"]}}"#,
            )
            .unwrap();
        let declared = env.requested_spec_hash();
        assert_ne!(declared, defaults);
        assert_eq!(read_spec(&store).unwrap().python, Some(vec!["3.13".into()]));
        // Other settings and key order do not change the requested declaration.
        store
            .write_text(
                "config.json",
                r#"{"environment":{"python":["3.13"],"version":1},"version":2,"appearance":{"theme":"dark"}}"#,
            )
            .unwrap();
        assert_eq!(env.requested_spec_hash(), declared);
        store
            .write_text(
                "config.json",
                r#"{"version":2,"environment":{"version":1,"python":["3.13","3.13"]}}"#,
            )
            .unwrap();
        assert!(read_spec(&store).unwrap_err().message.starts_with("environment.python"));
        store.write_text("config.json", r#"{"version":2,"environment":{"version":1,"python":"3.13"}}"#).unwrap();
        assert!(read_spec(&store).is_err());
        assert_ne!(env.requested_spec_hash(), declared);
    }
}
