//! One optional-install demand per verified environment generation.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::BackendKind,
    ports::AgentTools,
};
use std::sync::Mutex;
use workflow_environment::tools::{ToolPhase, ToolsStatus};
#[derive(Default)]
struct Demand {
    claude: bool,
    requested: bool,
}
#[derive(Default)]
pub struct ToolPolicy {
    demand: Mutex<Demand>,
}
impl ToolPolicy {
    pub fn environment_changed(&self) {
        *self.demand.lock().unwrap() = Demand::default();
    }
    pub fn refresh(
        &self,
        tools: &dyn AgentTools,
        default: BackendKind,
        required: Option<BackendKind>,
    ) -> Result<ToolsStatus> {
        let mut demand = self.demand.lock().unwrap();
        demand.claude |= required == Some(BackendKind::Claude);
        let mut status = tools.status()?;
        let absent = status
            .tools
            .iter()
            .any(|t| t.id == "claude" && t.phase == ToolPhase::NotInstalled);
        if (default == BackendKind::Claude || demand.claude) && absent && !demand.requested {
            demand.requested = true; // Durable job receipt may be lost; never create a retry storm.
            status = tools.install_claude(false)?;
        }
        for t in &status.tools {
            if matches!(t.id.as_str(), "codex" | "claude") && t.phase == ToolPhase::Ready {
                let expected = format!("/opt/workflow/tools/{}/bin/{}", t.id, t.id);
                if t.binary.as_deref() != Some(&expected) {
                    return Err(ChatError::new(
                        ErrorKind::BackendUnavailable,
                        "Unexpected managed agent executable",
                    ));
                }
            }
        }
        Ok(status)
    }
}
