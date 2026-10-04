//! How Codex App-Server runs inside the environment.
use crate::{model::PermissionPreset, ports::SpawnSpec, service::launch_env::filtered};
use std::{collections::BTreeMap, time::Duration};

pub const EXECUTABLE: &str = "/opt/workflow/tools/codex/bin/codex";

/// Bounds of the Codex login state machine (docs/engine/chat.md).
#[derive(Clone, Debug)]
pub struct LoginTimings {
    /// First account poll while waiting; doubles after a failure up to `max_poll`.
    pub poll: Duration,
    pub max_poll: Duration,
    /// A waiting attempt fails after this deadline and its server login is cancelled.
    pub timeout: Duration,
    /// How long a caller waits for the shared `account/read`. Longer than Codex 0.157.1's 15 s
    /// workspace-routing discovery, which an authenticated read can wait for.
    pub read_timeout: Duration,
    /// Reads that confirm a successful completion.
    pub confirm: Vec<Duration>,
    pub cancel_timeout: Duration,
}
impl Default for LoginTimings {
    fn default() -> Self {
        Self {
            poll: Duration::from_secs(2),
            max_poll: Duration::from_secs(15),
            timeout: Duration::from_secs(15 * 60),
            read_timeout: Duration::from_secs(30),
            confirm: [0, 500, 1000, 2000, 4000].map(Duration::from_millis).to_vec(),
            cancel_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CodexConfig {
    /// argv[0] inside the environment.
    pub executable: String,
    /// The Engine-selected guest `CODEX_HOME`, so a desktop `config.toml` never leaks in.
    pub codex_home: String,
    pub cwd: String,
    /// Guest base environment; filtered before use.
    pub base_env: BTreeMap<String, String>,
    pub args: Vec<String>,
    pub client_name: String,
    pub client_title: String,
    pub client_version: String,
    pub default_permissions: PermissionPreset,
    pub opt_out_notifications: Vec<String>,
    pub history_page_size: u32,
    pub timings: LoginTimings,
}
impl CodexConfig {
    pub fn new(executable: &str, codex_home: &str) -> Self {
        Self {
            executable: executable.into(),
            codex_home: codex_home.into(),
            cwd: "/workspace".into(),
            base_env: BTreeMap::new(),
            args: vec!["app-server".into()],
            client_name: "workflow".into(),
            client_title: "Workflow".into(),
            client_version: "1.0.0".into(),
            default_permissions: PermissionPreset::Ask,
            opt_out_notifications: Vec::new(),
            history_page_size: 20,
            timings: LoginTimings::default(),
        }
    }
}

pub fn spec(config: &CodexConfig) -> SpawnSpec {
    let mut env = filtered(&config.base_env);
    env.insert("CODEX_HOME".into(), config.codex_home.clone());
    let mut argv = vec![config.executable.clone()];
    argv.extend(config.args.iter().cloned());
    SpawnSpec {
        argv,
        cwd: config.cwd.clone(),
        env,
        label: "codex".into(),
    }
}
