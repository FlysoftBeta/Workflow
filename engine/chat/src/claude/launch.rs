use crate::{
    error::{ChatError, ErrorKind, Result},
    model::PermissionPreset,
    ports::SpawnSpec,
    service::launch_env::{Secret, filtered},
};
use std::collections::BTreeMap;
pub const EXECUTABLE: &str = "/opt/workflow/tools/claude/bin/claude";
pub enum Session<'a> {
    New(&'a str),
    Resume(&'a str),
    Fork {
        from: &'a str,
        id: &'a str,
        at_message: Option<&'a str>,
    },
}
pub fn permission_mode(preset: PermissionPreset) -> &'static str {
    match preset {
        PermissionPreset::Ask => "default",
        PermissionPreset::AutoEdit => "acceptEdits",
        PermissionPreset::Plan => "plan",
        PermissionPreset::DenyUnlisted => "dontAsk",
    }
}
pub fn spec(
    base: &BTreeMap<String, String>,
    credentials: &BTreeMap<String, Secret>,
    session: Session<'_>,
    model: Option<&str>,
    effort: Option<&str>,
    preset: PermissionPreset,
    extra: &[String],
) -> Result<SpawnSpec> {
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
    let mut argv = vec![
        EXECUTABLE.into(),
        "-p".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--include-partial-messages".into(),
        "--replay-user-messages".into(),
        "--permission-prompt-tool".into(),
        "stdio".into(),
        "--permission-mode".into(),
        permission_mode(preset).into(),
    ];
    if let Some(model) = model {
        argv.extend(["--model".into(), model.into()]);
    }
    if let Some(effort) = effort {
        argv.extend(["--effort".into(), effort.into()]);
    }
    let id = match session {
        Session::New(id) => {
            argv.extend(["--session-id".into(), id.into()]);
            id
        }
        Session::Resume(id) => {
            argv.extend(["--resume".into(), id.into()]);
            id
        }
        Session::Fork {
            from,
            id,
            at_message,
        } => {
            argv.extend([
                "--resume".into(),
                from.into(),
                "--fork-session".into(),
                "--session-id".into(),
                id.into(),
            ]);
            if let Some(at) = at_message {
                argv.extend(["--resume-session-at".into(), at.into()]);
            }
            id
        }
    };
    argv.extend(extra.iter().cloned());
    let mut env = filtered(base);
    for (key, value) in [
        ("CLAUDE_CONFIG_DIR", "/home/work/.claude"),
        ("TMPDIR", "/tmp"),
        ("CLAUDE_CODE_TMPDIR", "/tmp"),
        ("DISABLE_AUTOUPDATER", "1"),
        ("DISABLE_TELEMETRY", "1"),
        ("DISABLE_ERROR_REPORTING", "1"),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
    ] {
        env.insert(key.into(), value.into());
    }
    for (key, value) in credentials {
        env.insert(key.clone(), value.0.clone());
    }
    Ok(SpawnSpec {
        argv,
        env,
        cwd: "/workspace".into(),
        label: format!("claude:{id}"),
    })
}
pub fn transcript_path(cwd: &str, session: &str) -> Result<String> {
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
    Ok(format!(
        "/home/work/.claude/projects/{directory}/{session}.jsonl"
    ))
}
