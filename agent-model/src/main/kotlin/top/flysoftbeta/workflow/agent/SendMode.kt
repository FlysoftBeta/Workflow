package top.flysoftbeta.workflow.agent

import kotlinx.serialization.Serializable

@Serializable
enum class SendMode {
    /** Start a turn when idle, otherwise queue behind the running turn. */
    AUTO,
    START,
    /** Inject into the running turn (Codex `turn/steer`; Claude: priority `now`). */
    STEER,
    /** Run after the current turn (Codex `thread/queue/add`; Claude: queued user message). */
    QUEUE,
}
