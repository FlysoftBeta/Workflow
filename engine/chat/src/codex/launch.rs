use crate::{ports::SpawnSpec, service::launch_env::filtered};
use std::collections::BTreeMap;
pub fn spec(base: &BTreeMap<String, String>) -> SpawnSpec {
    let mut env = filtered(base);
    env.insert("CODEX_HOME".into(), "/home/work/.codex".into());
    SpawnSpec {
        argv: vec![
            "/opt/workflow/tools/codex/bin/codex".into(),
            "app-server".into(),
        ],
        cwd: "/workspace".into(),
        env,
        label: "codex".into(),
    }
}
