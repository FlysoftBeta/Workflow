use crate::{
    layout::s,
    protocol::{Error, Result},
    storage::{self, now},
};
use serde_json::{Value as V, json};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::Read,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
const ENVCTL: &str = "/usr/local/libexec/workflow/envctl";
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
    pub state: V,
    pub building: bool,
    read_only: bool,
    pub running: Arc<AtomicUsize>,
}
impl Environment {
    pub fn load(options: Options, running: Arc<AtomicUsize>) -> Result<Self> {
        let dir = options.root.join(".workspace/environment");
        fs::create_dir_all(dir.join("generations"))?;
        fs::create_dir_all(dir.join("stores"))?;
        let path = dir.join("environment.json");
        let fresh = || json!({"format":1,"active":null,"pending":null,"failure":null,"status":"unavailable","stage":"idle","step":0,"steps":0});
        let mut read_only = false;
        let mut state = if path.exists() {
            match storage::read_json(&path) {
                Ok(v) if v["format"] == 1 => v,
                Ok(_) => {
                    read_only = true;
                    let mut v = fresh();
                    v["status"] = json!("failed");
                    v["failure"] = json!({"stage":"recovery","message":"Unsupported environment state; preserved read-only"});
                    v
                }
                Err(_) => {
                    let corrupt = options.root.join(".workspace/corrupt");
                    fs::create_dir_all(&corrupt)?;
                    fs::rename(
                        &path,
                        corrupt.join(format!("{}-environment.json", uuid::Uuid::new_v4())),
                    )?;
                    let mut v = storage::read_json(&dir.join("environment.json.bak"))
                        .ok()
                        .filter(|v| v["format"] == 1)
                        .unwrap_or_else(fresh);
                    v["status"] = json!("failed");
                    v["failure"] = json!({"stage":"recovery","message":"Damaged environment state preserved; last valid state restored when available"});
                    v
                }
            }
        } else {
            fresh()
        };
        if state["status"] == "applying" {
            if let Some(job) = state["jobId"]
                .as_str()
                .filter(|id| storage::identifier(id).is_ok())
            {
                crate::home_stage::cleanup(&dir.join("generations").join(job));
            }
            state["status"] = json!(if state["active"].is_object() {
                "ready"
            } else {
                "unavailable"
            });
            state["failure"] = json!({"stage":"interrupted","message":"Previous build was interrupted; retry to rebuild"});
        }
        let out = Self {
            options,
            state,
            building: false,
            read_only,
            running,
        };
        if !out.read_only {
            out.persist()?;
        }
        Ok(out)
    }
    fn directory(&self) -> PathBuf {
        self.options.root.join(".workspace/environment")
    }
    fn requested_spec_hash(&self) -> V {
        let path = self.options.root.join(".workspace/env.json");
        let read = (|| -> std::io::Result<String> {
            let file = File::open(path)?;
            let size = file.metadata()?.len();
            let mut bytes = Vec::new();
            file.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
            Ok(format!("{size}:{}", storage::hash(&bytes)))
        })();
        json!(read.unwrap_or_else(|error| format!("unreadable:{:?}", error.kind())))
    }
    /// Saved or externally edited declarations are reconciled by the owner, including after reconnect.
    /// Recording the attempted input also prevents malformed/failed configurations from retrying forever.
    pub fn reconcile_changed(shared: &Arc<Mutex<Self>>) {
        let changed = {
            let e = shared.lock().unwrap();
            !e.read_only && !e.building && e.options.runtime.is_some()
                && (e.options.apk.is_some() || e.options.image.is_some())
                // Files/proxy-only connections need no extracted runtime. The first explicit
                // reconcile enrolls this environment in automatic declaration monitoring.
                && (e.state.get("requestedSpecHash").is_some() || e.state["active"].is_object())
                && e.state["requestedSpecHash"] != e.requested_spec_hash()
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
        let path = self.directory().join("environment.json");
        if path.exists() {
            storage::atomic(
                &self.directory().join("environment.json.bak"),
                &fs::read(&path)?,
            )?;
        }
        storage::write_json(&path, &self.state)
    }
    pub fn status(&self) -> V {
        let mut status = self.state.clone();
        status["runningProcesses"] = json!(self.running.load(Ordering::SeqCst));
        status["environmentAvailable"] = json!(self.state["active"].is_object());
        status["usable"] = json!(self.state["active"].is_object());
        status["phase"] = json!(match s(&self.state["status"]) {
            "ready" => "ready",
            "pending_restart" => "needs_restart",
            "failed" => "failed",
            "applying" =>
                if s(&self.state["stage"]) == "install" || s(&self.state["stage"]) == "image" {
                    "installing"
                } else {
                    "building"
                },
            _ => "not_installed",
        });
        status["progress"] = if self.state["steps"].as_f64().unwrap_or(0.0) > 0.0 {
            json!(
                self.state["step"].as_f64().unwrap_or(0.0) / self.state["steps"].as_f64().unwrap()
            )
        } else {
            V::Null
        };
        status["error"] = self.state["failure"]["message"].clone();
        status["architecture"] = json!(if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "amd64"
        });
        status
    }
    pub fn reconcile(shared: &Arc<Mutex<Self>>, retry: bool) -> Result<V> {
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
            e.state["requestedSpecHash"] = e.requested_spec_hash();
            spec = match storage::read_json(&e.options.root.join(".workspace/env.json")).and_then(
                |v| {
                    validate(&v)?;
                    Ok(v)
                },
            ) {
                Ok(v) => v,
                Err(error) => {
                    e.state["status"] = json!("failed");
                    e.state["stage"] = json!("config");
                    e.state["failure"] = json!({"stage":"config","message":error.message});
                    e.persist()?;
                    return Ok(e.status());
                }
            };
            opts = e.options.clone();
            active = e.state["active"].clone();
            job = uuid::Uuid::new_v4().to_string();
            running = e.running.clone();
            let fingerprint = storage::hash(&serde_json::to_vec(&spec).unwrap());
            if !retry && e.state["failure"]["fingerprint"] == fingerprint {
                return Ok(e.status());
            }
            e.state["status"] = json!("applying");
            e.state["stage"] = json!("image");
            e.state["jobId"] = json!(job);
            e.state["step"] = json!(0);
            e.state["steps"] = json!(0);
            if let Err(error) = e.persist() {
                e.state["status"] = json!("failed");
                e.state["failure"] = json!({"stage":"persist","message":error.message});
                return Err(error);
            }
            e.building = true;
        }
        let shared2 = shared.clone();
        std::thread::spawn(move || {
            let fingerprint = storage::hash(&serde_json::to_vec(&spec).unwrap());
            let outcome = build(&opts, &mut spec, &active, &job, &shared2);
            if outcome.is_err() {
                crate::home_stage::cleanup(
                    &opts
                        .root
                        .join(".workspace/environment/generations")
                        .join(&job),
                );
            }
            let current_fingerprint = storage::read_json(&opts.root.join(".workspace/env.json"))
                .ok()
                .map(|v| storage::hash(&serde_json::to_vec(&v).unwrap()));
            let mut e = shared2.lock().unwrap();
            if current_fingerprint.as_deref() != Some(&fingerprint) {
                e.building = false;
                e.state["status"] = json!(if e.state["active"].is_object() {
                    "ready"
                } else {
                    "unavailable"
                });
                e.state["stage"] = json!("configuration_changed");
                let _ = e.persist();
                crate::home_stage::cleanup(
                    &opts
                        .root
                        .join(".workspace/environment/generations")
                        .join(&job),
                );
                drop(e);
                let _ = Self::reconcile(&shared2, false);
                return;
            }
            e.building = false;
            match outcome {
                Ok(activation) => {
                    e.state["failure"] = V::Null;
                    if e.state["active"] == activation {
                        e.state["pending"] = V::Null;
                        e.state["status"] = json!("ready");
                    } else if running.load(Ordering::SeqCst) > 0 {
                        e.state["pending"] = activation;
                        e.state["status"] = json!("pending_restart");
                    } else {
                        match activate(&opts, &activation) {
                            Ok(()) => {
                                e.state["previous"] = e.state["active"].clone();
                                e.state["active"] = activation;
                                e.state["pending"] = V::Null;
                                e.state["status"] = json!("ready");
                            }
                            Err(err) => {
                                crate::home_stage::cleanup(
                                    &opts
                                        .root
                                        .join(".workspace/environment/generations")
                                        .join(&job),
                                );
                                e.state["status"] = json!("failed");
                                e.state["failure"] = json!({"stage":"activate","message":err.message,"fingerprint":fingerprint});
                            }
                        }
                    }
                }
                Err(err) => {
                    e.state["status"] = json!("failed");
                    e.state["failure"] = json!({"stage":e.state["stage"],"message":err.message,"fingerprint":fingerprint});
                }
            }
            if let Err(err) = e.persist() {
                e.state["status"] = json!("failed");
                e.state["failure"] = json!({"stage":"persist","message":err.message});
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
        if !self.state["pending"].is_object() {
            return Ok(false);
        }
        verify_generation(&self.options, &self.state["pending"])?;
        Ok(true)
    }
    pub fn restart(&mut self) -> Result<V> {
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
        let activation = if self.state["pending"].is_object() {
            self.state["pending"].clone()
        } else {
            self.state["active"].clone()
        };
        if !activation.is_object() {
            return Err(Error::business("unavailable", "no verified environment"));
        }
        verify_generation(&self.options, &activation)?;
        if let Err(error) = activate(&self.options, &activation) {
            crate::home_stage::cleanup(
                &self
                    .directory()
                    .join("generations")
                    .join(crate::workspace::required(&activation, "generation")?),
            );
            self.state["pending"] = V::Null;
            self.state["status"] = json!("failed");
            self.state["failure"] = json!({"stage":"activate","message":error.message});
            self.persist()?;
            return Err(error);
        }
        if self.state["active"] != activation {
            self.state["previous"] = self.state["active"].clone();
        }
        self.state["active"] = activation;
        self.state["pending"] = V::Null;
        self.state["status"] = json!("ready");
        self.persist()?;
        Ok(self.status())
    }
    pub fn process_command(&self, argv: &[String], cwd: &str, env: &V) -> Result<Command> {
        if !self.state["active"].is_object() {
            return Err(Error::business(
                "environment_unavailable",
                "no verified environment has been activated",
            ));
        }
        if !cwd.starts_with('/') || cwd.contains('\0') || cwd.split('/').any(|s| s == "..") {
            return Err(Error::invalid(
                "cwd must be a normalized guest absolute path",
            ));
        }
        let active = &self.state["active"];
        let generation = self
            .directory()
            .join("generations")
            .join(storage::identifier(crate::workspace::required(
                active,
                "generation",
            )?)?);
        let mut c = guest_command(
            &self.options,
            &generation,
            "work",
            cwd,
            &active["environment"],
            true,
        )?;
        if let Some(map) = env.as_object() {
            for (k, v) in map {
                if !env_key(k) || !v.is_string() {
                    return Err(Error::invalid("invalid process environment"));
                }
                c.arg(format!("{k}={}", s(v)));
            }
        }
        c.args(argv);
        Ok(c)
    }
}
pub fn validate(v: &V) -> Result<()> {
    if !v.is_object() || v["version"] != 1 {
        return Err(Error::invalid("env.version must be 1"));
    }
    for lang in ["python", "node"] {
        if let Some(list) = v.get(lang) {
            let Some(list) = list.as_array() else {
                return Err(Error::invalid(&format!("{lang}: expected version array")));
            };
            let mut seen = HashSet::new();
            for version in list {
                let Some(version) = version.as_str() else {
                    return Err(Error::invalid("expected version string"));
                };
                let parts: Vec<_> = version.split('.').collect();
                if parts.is_empty()
                    || parts.len() > 3
                    || parts.iter().any(|p| {
                        p.is_empty()
                            || !p.bytes().all(|b| b.is_ascii_digit())
                            || (p.len() > 1 && p.starts_with('0'))
                    })
                    || parts.iter().any(|p| p.parse::<u16>().is_err())
                    || (lang == "python" && parts[0] != "3")
                    || (lang == "node" && parts[0].parse::<u16>().unwrap_or(0) < 1)
                    || !seen.insert(version)
                {
                    return Err(Error::invalid(&format!(
                        "{lang}: duplicate or invalid version"
                    )));
                }
            }
        }
    }
    if let Some(pkgs) = v.get("packages") {
        let Some(pkgs) = pkgs.as_array() else {
            return Err(Error::invalid("packages: expected array"));
        };
        let mut seen = HashSet::new();
        for p in pkgs {
            let p = s(p);
            if p.is_empty()
                || !p.bytes().all(|b| {
                    b.is_ascii_lowercase()
                        || b.is_ascii_digit()
                        || matches!(b, b'+' | b'-' | b'.' | b':')
                })
                || !p.as_bytes()[0].is_ascii_alphanumeric()
                || !seen.insert(p)
            {
                return Err(Error::invalid("packages: invalid or duplicate package"));
            }
        }
    }
    if let Some(env) = v.get("env") {
        let Some(env) = env.as_object() else {
            return Err(Error::invalid("env: expected object"));
        };
        for (k, val) in env {
            if !env_key(k) || !val.is_string() || s(val).contains('\0') {
                return Err(Error::invalid("env: invalid variable"));
            }
        }
    }
    if let Some(scripts) = v.get("post_scripts") {
        let Some(scripts) = scripts.as_array() else {
            return Err(Error::invalid("post_scripts: expected array"));
        };
        let mut seen = HashSet::new();
        for script in scripts {
            let id = crate::workspace::required(script, "id")?;
            storage::identifier(id)?;
            if !seen.insert(id)
                || !script["run"].is_string()
                || s(&script["run"]).trim().is_empty()
                || s(&script["run"]).contains('\0')
                || !matches!(s(&script["user"]), "work" | "root")
            {
                return Err(Error::invalid(
                    "post_scripts: duplicate id or invalid run/user",
                ));
            }
        }
    }
    Ok(())
}
fn env_key(k: &str) -> bool {
    !k.is_empty()
        && (k.as_bytes()[0].is_ascii_alphabetic() || k.starts_with('_'))
        && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn runtime(opts: &Options) -> Result<&Path> {
    opts.runtime
        .as_deref()
        .filter(|p| p.is_file())
        .ok_or_else(|| {
            Error::business(
                "runtime_unavailable",
                "workspace runtime executable is not configured",
            )
        })
}
fn guest_command(
    opts: &Options,
    generation: &Path,
    user: &str,
    cwd: &str,
    env: &V,
    interactive: bool,
) -> Result<Command> {
    let mut c = Command::new(runtime(opts)?);
    c.arg("run")
        .arg("--root")
        .arg(generation.join("rootfs"))
        .arg("--user")
        .arg(user)
        .arg("--cwd")
        .arg(cwd);
    let sockets = std::env::temp_dir().join(format!(
        "wf-sock-{}",
        &storage::hash(opts.root.to_string_lossy().as_bytes())[..12]
    ));
    fs::create_dir_all(&sockets)?;
    c.arg("--socket-dir").arg(&sockets);
    if let Some(loader) = &opts.loader {
        c.arg("--loader").arg(loader);
    }
    let stores = opts.root.join(".workspace/environment/stores");
    let toolchains = stores.join("toolchains");
    fs::create_dir_all(&toolchains)?;
    c.arg("--bind")
        .arg(format!("{}:/opt/toolchains", toolchains.display()));
    if !interactive && generation.join("post-home").is_dir() {
        c.arg("--bind").arg(format!(
            "{}:/home/work",
            generation.join("post-home").display()
        ));
    }
    if interactive {
        let home = stores.join("home/work");
        fs::create_dir_all(&home)?;
        c.arg("--bind")
            .arg(format!("{}:/home/work", home.display()));
        c.arg("--bind")
            .arg(format!("{}:/workspace", opts.root.display()))
            .arg("--hide")
            .arg("/workspace/.workspace/state")
            .arg("--hide")
            .arg("/workspace/.workspace/environment")
            .arg("--hide")
            .arg("/workspace/.workspace/documents")
            .arg("--hide")
            .arg("/workspace/.workspace/uploads")
            .arg("--hide")
            .arg("/workspace/.workspace/corrupt")
            .arg("--hide")
            .arg("/workspace/.workspace/trash")
            .arg("--hide")
            .arg("/workspace/.workspace/engine.lock");
    }
    let resolver = opts.root.join(".workspace/environment/network/resolv.conf");
    if resolver.is_file() {
        c.arg("--bind")
            .arg(format!("{}:/etc/resolv.conf", resolver.display()));
    }
    crate::tools::bind(opts, &mut c)?;
    c.env_clear();
    c.env("PATH","/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin").env("HOME",if user=="root"{"/root"}else{"/home/work"}).env("USER",user).env("LANG","C.UTF-8").env("TERM","xterm-256color").env("NVM_DIR","/opt/toolchains/nvm").env("UV_PYTHON_INSTALL_DIR","/opt/toolchains/uv/python").env("UV_CACHE_DIR","/opt/toolchains/uv/cache").env("TMPDIR","/tmp");
    // Apply caller variables with the guest env executable. Loader variables must
    // never affect the host runtime before it has entered the environment.
    c.arg("--").arg("/usr/bin/env");
    if let Some(vars) = env.as_object() {
        for (k, v) in vars {
            c.arg(format!("{k}={}", s(v)));
        }
    }
    Ok(c)
}
fn checked(mut c: Command, label: &str) -> Result<Vec<u8>> {
    unsafe {
        c.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let err_reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut pipe = stderr;
        let mut buf = [0u8; 8192];
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            if data.len() < 65536 {
                let take = n.min(65536 - data.len());
                data.extend_from_slice(&buf[..take]);
            }
        }
    });
    let out_reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut pipe = stdout;
        let mut buf = [0u8; 8192];
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            if data.len() < 1024 * 1024 {
                let take = n.min(1024 * 1024 - data.len());
                data.extend_from_slice(&buf[..take]);
            }
        }
        data
    });
    let status = child.wait()?;
    let _ = err_reader.join();
    let output = out_reader.join().unwrap_or_default();
    if !status.success() {
        return Err(Error::business(
            "build_failed",
            &format!("{label} exited with {}", status.code().unwrap_or(128)),
        ));
    }
    Ok(output)
}
fn progress(shared: &Arc<Mutex<Environment>>, stage: &str, step: usize, steps: usize) {
    let mut e = shared.lock().unwrap();
    e.state["stage"] = json!(stage);
    e.state["step"] = json!(step);
    e.state["steps"] = json!(steps);
    let _ = e.persist();
}
fn image(opts: &Options) -> Result<(PathBuf, PathBuf, V)> {
    let (image, index) = if let (Some(image), Some(index)) = (&opts.image, &opts.image_index) {
        (image.clone(), index.clone())
    } else if let Some(apk) = &opts.apk {
        let dir = opts.root.join(".workspace/environment/image");
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
        let metadata = crate::protocol::strict_json(&index_bytes)
            .map_err(|_| Error::business("invalid_image", "invalid image index"))?;
        let sha = crate::workspace::required(&metadata, "sha256")?;
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
        storage::atomic(&index, &index_bytes)?;
        (image, index)
    } else {
        return Err(Error::business(
            "image_unavailable",
            "customized image and index or APK are required",
        ));
    };
    let metadata = storage::read_json(&index)?;
    let m = &metadata["metadata"];
    if m["format"] != "workflow-image"
        || m["formatVersion"] != 2
        || m["profile"] != "workspace"
        || m["type"] != "debian-trixie"
        || m["typeVersion"] != 1
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
    spec: &mut V,
    active: &V,
    job: &str,
    shared: &Arc<Mutex<Environment>>,
) -> Result<V> {
    let (image, index, metadata) = image(opts)?;
    for lang in ["python", "node"] {
        if spec.get(lang).is_none() {
            let d = &metadata["metadata"]["defaults"][lang];
            spec[lang] = if d.is_array() {
                d.clone()
            } else if d.is_string() {
                json!([d])
            } else {
                return Err(Error::business(
                    "invalid_image",
                    "language defaults missing",
                ));
            };
        }
    }
    for (k, v) in [
        ("packages", json!([])),
        ("env", json!({})),
        ("post_scripts", json!([])),
    ] {
        if spec.get(k).is_none() {
            spec[k] = v;
        }
    }
    validate(spec)?;
    let image_sha = crate::workspace::required(&metadata, "sha256")?;
    let fingerprint = storage::hash(&serde_json::to_vec(spec).unwrap());
    if active["fingerprint"] == fingerprint && active["imageSha256"] == image_sha {
        verify_generation(opts, active)?;
        return Ok(active.clone());
    }
    let generation = opts
        .root
        .join(".workspace/environment/generations")
        .join(job);
    let total = 4
        + spec["python"].as_array().unwrap().len()
        + spec["node"].as_array().unwrap().len()
        + spec["post_scripts"].as_array().unwrap().len();
    let mut step = 1;
    progress(shared, "install", step, total);
    let mut install = Command::new(runtime(opts)?);
    install
        .arg("install")
        .arg("--image")
        .arg(image)
        .arg("--index")
        .arg(index)
        .arg("--target")
        .arg(&generation)
        .arg("--profile")
        .arg("workspace")
        .arg("--quiet");
    checked(install, "image install")?;
    let stores = opts.root.join(".workspace/environment/stores");
    if let Some(defs) = metadata["metadata"]["stores"].as_object() {
        for (_, seed) in defs {
            let name = crate::workspace::required(seed, "store")?;
            if name.starts_with('/')
                || name
                    .split('/')
                    .any(|s| s.is_empty() || s == ".." || s == ".")
            {
                return Err(Error::business("invalid_image", "invalid seed store path"));
            }
            let target = stores.join(name);
            if s(&seed["seed"]) != "if-absent" || !target.exists() {
                merge_seed(&generation.join("seeds").join(name), &target)?;
            }
        }
    }
    let mut merged_env = metadata["metadata"]["environment"].clone();
    if !merged_env.is_object() {
        merged_env = json!({});
    }
    storage::merge(&mut merged_env, &spec["env"]);
    let process_env = merged_env.clone();
    let packages = spec["packages"].as_array().unwrap();
    step += 1;
    progress(shared, "packages", step, total);
    if !packages.is_empty() {
        let mut cmd = guest_command(opts, &generation, "root", "/", &merged_env, false)?;
        cmd.arg(ENVCTL)
            .arg("apt-install")
            .args(packages.iter().map(s));
        checked(cmd, "package install")?;
    }
    for lang in ["python", "node"] {
        for version in spec[lang].as_array().unwrap() {
            step += 1;
            progress(shared, lang, step, total);
            let mut cmd =
                guest_command(opts, &generation, "work", "/home/work", &merged_env, false)?;
            cmd.arg(ENVCTL).arg(lang).arg(s(version));
            checked(cmd, &format!("{lang} install"))?;
        }
    }
    step += 1;
    progress(shared, "verify", step, total);
    let mut cmd = guest_command(opts, &generation, "work", "/home/work", &merged_env, false)?;
    cmd.arg(ENVCTL)
        .arg("verify-many")
        .arg(spec["python"].to_string())
        .arg(spec["node"].to_string())
        .args(packages.iter().map(s));
    let bytes = checked(cmd, "toolchain verification")?;
    let verified = crate::protocol::strict_json(&bytes).map_err(|_| {
        Error::business(
            "verification_failed",
            "invalid toolchain verification response",
        )
    })?;
    let profile = crate::workspace::required(&verified, "profile")?;
    storage::identifier(profile)?;
    for lang in ["python", "node"] {
        let expected = spec[lang].as_array().unwrap();
        let actual = verified[lang]
            .as_array()
            .ok_or_else(|| Error::business("verification_failed", "language array missing"))?;
        if actual.len() != expected.len()
            || !expected.iter().zip(actual).all(|(want, got)| {
                s(got) == s(want) || s(got).starts_with(&format!("{}.", s(want)))
            })
        {
            return Err(Error::business(
                "verification_failed",
                "installed language versions do not match configuration",
            ));
        }
    }
    if packages
        .iter()
        .any(|p| !verified["packages"][s(p)].is_string())
    {
        return Err(Error::business(
            "verification_failed",
            "required packages missing",
        ));
    }
    verify_tools(opts, &generation, &merged_env)?;
    // Scripts see this generation's verified default toolchain without changing the active profile.
    merged_env["PATH"] = json!(format!(
        "/opt/toolchains/profiles/{profile}/python/bin:/opt/toolchains/profiles/{profile}/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    ));
    if !spec["post_scripts"].as_array().unwrap().is_empty() {
        crate::home_stage::prepare(&opts.root, &generation)?;
    }
    for script in spec["post_scripts"].as_array().unwrap() {
        step += 1;
        progress(
            shared,
            &format!("post_script:{}", s(&script["id"])),
            step,
            total,
        );
        let mut cmd = guest_command(
            opts,
            &generation,
            s(&script["user"]),
            "/",
            &merged_env,
            false,
        )?;
        cmd.arg("/bin/bash").arg("-euc").arg(s(&script["run"]));
        checked(cmd, &format!("post script {}", s(&script["id"])))?;
    }
    let activation = json!({"generation":job,"profile":profile,"config":spec,"environment":process_env,"fingerprint":fingerprint,"imageSha256":image_sha,"verified":verified,"verifiedAt":now()});
    storage::write_json(&generation.join("workspace-activation.json"), &activation)?;
    step += 1;
    progress(shared, "activate", step, total);
    verify_generation(opts, &activation)?;
    Ok(activation)
}
fn verify_tools(opts: &Options, generation: &Path, env: &V) -> Result<()> {
    for (argv, version) in crate::tools::verification_commands(opts)? {
        let mut command = guest_command(opts, generation, "work", "/home/work", env, false)?;
        command.args(&argv);
        let output = checked(command, "required tool verification")?;
        if !String::from_utf8_lossy(&output).contains(&version) {
            return Err(Error::business(
                "tool_version_mismatch",
                "Required Engine tool version does not match its pin",
            ));
        }
    }
    Ok(())
}
fn verify_generation(opts: &Options, activation: &V) -> Result<()> {
    let id = storage::identifier(crate::workspace::required(activation, "generation")?)?;
    let generation = opts
        .root
        .join(".workspace/environment/generations")
        .join(id);
    let mut cmd = Command::new(runtime(opts)?);
    cmd.arg("verify")
        .arg("--generation")
        .arg(&generation)
        .arg("--quiet");
    checked(cmd, "generation verification")?;
    verify_tools(opts, &generation, &activation["environment"])?;
    Ok(())
}
fn activate(opts: &Options, activation: &V) -> Result<()> {
    let id = storage::identifier(crate::workspace::required(activation, "generation")?)?;
    let generation = opts
        .root
        .join(".workspace/environment/generations")
        .join(id);
    let mut cmd = guest_command(
        opts,
        &generation,
        "work",
        "/home/work",
        &activation["config"]["env"],
        false,
    )?;
    cmd.arg(ENVCTL)
        .arg("activate")
        .arg(crate::workspace::required(activation, "profile")?);
    let active_link = opts
        .root
        .join(".workspace/environment/stores/toolchains/active");
    let previous = fs::read_link(&active_link).ok();
    checked(cmd, "profile activation")?;
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

#[cfg(test)]
mod launcher_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn bundled_cli_launcher_overlays_existing_image_entry_point() {
        let temp = tempfile::tempdir().unwrap();
        let opts = Options {
            root: temp.path().to_path_buf(),
            runtime: Some(PathBuf::from("/bin/true")),
            tools: Some(temp.path().join("payload")),
            ..Options::default()
        };
        crate::tools::fixture(&temp.path().join("payload"));
        let generation = temp.path().join("generation");
        let command =
            guest_command(&opts, &generation, "work", "/workspace", &json!({}), true).unwrap();
        let launcher = temp.path().join(".workspace/environment/launchers/codex");
        let binding = format!("{}:/usr/local/bin/codex", launcher.display());
        assert!(command.get_args().any(|arg| arg == binding.as_str()));
        assert_eq!(
            fs::read(&launcher).unwrap(),
            include_bytes!("../guest/codex")
        );
        assert_eq!(
            fs::metadata(&launcher).unwrap().permissions().mode() & 0o777,
            0o755
        );
        // Repeated process launches reuse the managed script without changing the image.
        guest_command(&opts, &generation, "work", "/workspace", &json!({}), true).unwrap();
        assert!(!generation.exists());
    }
}
