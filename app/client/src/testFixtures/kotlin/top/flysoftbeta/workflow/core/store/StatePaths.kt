package top.flysoftbeta.workflow.core.store

import top.flysoftbeta.workflow.core.io.Hashing

/** Frozen reference-store paths for tests; production disk layout belongs to Rust. */
object StatePaths {
    const val ROOT = ".workspace/state"
    const val SESSIONS = "$ROOT/sessions.json"
    const val SESSIONS_BACKUP = "$ROOT/sessions.json.bak"
    const val DRAFTS = "$ROOT/drafts"
    const val COMPOSERS = "$ROOT/composers"
    const val CONFIG_LAST_GOOD = "$ROOT/config.last-good.json"
    const val CONVERSATIONS = "$ROOT/conversations.json"
    const val CORRUPT = "$ROOT/corrupt"

    fun draft(path: String) = "$DRAFTS/${Hashing.sha256(path).take(40)}.json"
    fun composer(conversationId: String) = "$COMPOSERS/${Hashing.sha256(conversationId).take(40)}.json"
}
