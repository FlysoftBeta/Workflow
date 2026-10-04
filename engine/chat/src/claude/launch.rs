//! Claude Code launch contract: argv, filtered environment and the permission-mode guard.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::PermissionPreset,
    ports::SpawnSpec,
    service::launch_env::{Secret, filtered},
};
use std::collections::BTreeMap;
pub const EXECUTABLE: &str = "/opt/workflow/tools/claude/bin/claude";
/// Modes in which nothing but the user approves. `bypassPermissions` and `auto` are refused.
pub const ALLOWED_MODES: [&str; 4] = ["default", "acceptEdits", "plan", "dontAsk"];

/// How the CLI runs. Paths are guest paths.
#[derive(Clone, Debug)]
pub struct LaunchConfig {
    pub executable: String,
    /// `CLAUDE_CONFIG_DIR`: credentials and transcripts.
    pub config_dir: String,
    pub tmp_dir: String,
    /// Guest base environment; filtered before use.
    pub base_env: BTreeMap<String, String>,
    /// Variables the Engine sets deliberately (an in-memory API key). Never logged.
    pub credentials: BTreeMap<String, Secret>,
    pub default_permissions: PermissionPreset,
    pub extra_args: Vec<String>,
}
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// How a process attaches to a session.
pub enum Session<'a> {
    New(&'a str),
    Resume(&'a str),
    /// Branch `from` into `id`; `at_message` keeps history up to that message.
    Fork {
        from: &'a str,
        id: &'a str,
        at_message: Option<&'a str>,
    },
}
impl Session<'_> {
    pub fn id(&self) -> &str {
        match self {
            Session::New(id) | Session::Resume(id) => id,
            Session::Fork { id, .. } => id,
        }
    }
}
pub fn permission_mode(preset: PermissionPreset) -> &'static str {
    match preset {
        PermissionPreset::Ask => "default",
        PermissionPreset::AutoEdit => "acceptEdits",
        PermissionPreset::Plan => "plan",
        PermissionPreset::DenyUnlisted => "dontAsk",
    }
}
fn check_extra(extra: &[String]) -> Result<()> {
    for arg in extra {
        let flag = arg.split('=').next().unwrap_or(arg);
        if matches!(
            flag,
            "--dangerously-skip-permissions"
                | "--allow-dangerously-skip-permissions"
                | "--permission-prompt-tool"
                | "--permission-mode"
        ) || matches!(arg.as_str(), "bypassPermissions" | "auto")
        {
            return Err(ChatError::new(
                ErrorKind::InvalidArgument,
                "extra arguments may not weaken permissions",
            ));
        }
    }
    Ok(())
}
pub fn argv(
    config: &LaunchConfig,
    session: &Session<'_>,
    model: Option<&str>,
    effort: Option<&str>,
    preset: Option<PermissionPreset>,
) -> Result<Vec<String>> {
    let mode = permission_mode(preset.unwrap_or(config.default_permissions));
    check_extra(&config.extra_args)?;
    let mut argv: Vec<String> = [
        config.executable.as_str(),
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--replay-user-messages",
        "--permission-prompt-tool",
        "stdio",
        "--permission-mode",
        mode,
    ]
    .map(String::from)
    .to_vec();
    if let Some(model) = model {
        argv.extend(["--model".into(), model.into()]);
    }
    if let Some(effort) = effort {
        argv.extend(["--effort".into(), effort.into()]);
    }
    match session {
        Session::New(id) => argv.extend(["--session-id".into(), id.to_string()]),
        Session::Resume(id) => argv.extend(["--resume".into(), id.to_string()]),
        Session::Fork {
            from,
            id,
            at_message,
        } => {
            argv.extend([
                "--resume".into(),
                from.to_string(),
                "--fork-session".into(),
                "--session-id".into(),
                id.to_string(),
            ]);
            if let Some(at) = at_message {
                argv.extend(["--resume-session-at".into(), at.to_string()]);
            }
        }
    }
    argv.extend(config.extra_args.iter().cloned());
    Ok(argv)
}
/// The child's complete environment: inherited credentials and loader injection are removed, the
/// updater and telemetry are disabled, and only deliberate Engine credentials are added.
pub fn env(config: &LaunchConfig) -> BTreeMap<String, String> {
    let mut env = filtered(&config.base_env);
    for (key, value) in [
        ("CLAUDE_CONFIG_DIR", config.config_dir.as_str()),
        ("TMPDIR", config.tmp_dir.as_str()),
        ("CLAUDE_CODE_TMPDIR", config.tmp_dir.as_str()),
        ("DISABLE_AUTOUPDATER", "1"),
        ("DISABLE_TELEMETRY", "1"),
        ("DISABLE_ERROR_REPORTING", "1"),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
    ] {
        env.insert(key.into(), value.into());
    }
    for (key, value) in &config.credentials {
        env.insert(key.clone(), value.0.clone());
    }
    env
}
pub fn spec(
    config: &LaunchConfig,
    cwd: &str,
    session: &Session<'_>,
    model: Option<&str>,
    effort: Option<&str>,
    preset: Option<PermissionPreset>,
) -> Result<SpawnSpec> {
    Ok(SpawnSpec {
        argv: argv(config, session, model, effort, preset)?,
        env: env(config),
        cwd: cwd.into(),
        label: format!("claude:{}", session.id()),
    })
}
/// `claude auth login`, run by the user in a terminal with the same environment.
pub fn login_argv(config: &LaunchConfig) -> Vec<String> {
    vec![config.executable.clone(), "auth".into(), "login".into()]
}
pub fn setup_token_argv(config: &LaunchConfig) -> Vec<String> {
    vec![config.executable.clone(), "setup-token".into()]
}
pub fn logout_argv(config: &LaunchConfig) -> Vec<String> {
    vec![config.executable.clone(), "auth".into(), "logout".into()]
}
/// `$CLAUDE_CONFIG_DIR/projects/<cwd with non-alphanumerics as "-">/<session>.jsonl`.
pub fn transcript_path(config_dir: &str, cwd: &str, session: &str) -> Result<String> {
    if session.is_empty()
        || session.len() > 200
        || !session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(ChatError::new(
            ErrorKind::InvalidArgument,
            "invalid Claude session ID",
        ));
    }
    let directory: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    Ok(format!("{config_dir}/projects/{directory}/{session}.jsonl"))
}
