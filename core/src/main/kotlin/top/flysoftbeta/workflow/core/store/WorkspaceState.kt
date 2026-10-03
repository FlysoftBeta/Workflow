package top.flysoftbeta.workflow.core.store

import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.resource.DiskVersion
import top.flysoftbeta.workflow.core.resource.Drafts
import top.flysoftbeta.workflow.core.resource.FileDraft
import top.flysoftbeta.workflow.core.resource.FileStatus
import top.flysoftbeta.workflow.core.resource.ResourceRef
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.session.SessionId

enum class StoreStatus { LOADING, READY, FAILED }

enum class NoticeKind {
    /** A damaged state file was set aside (bytes kept in `.workspace/corrupt/`). */
    RECOVERED,
    /** State was written by a newer app version and is kept read-only. */
    NEWER_FORMAT,
}

data class StoreNotice(val id: String, val kind: NoticeKind, val message: String)

/**
 * The single authoritative workspace state, published by [WorkspaceStore.state]. Unchanged parts
 * keep their instances between emissions, so `map { it.x }.distinctUntilChanged()` is cheap.
 */
data class WorkspaceState(
    val status: StoreStatus = StoreStatus.LOADING,
    val failure: String? = null,
    /** All sessions, including archived ones. */
    val sessions: List<Session> = emptyList(),
    val activeSessionId: SessionId? = null,
    /** Persistent sessions the user ordered by dragging (pinned above the automatic ranking). */
    val pinned: List<SessionId> = emptyList(),
    /** File Working Resources by normalized path. */
    val drafts: Map<String, FileDraft> = emptyMap(),
    /** Composer drafts by conversation id; includes empty revision tombstones. */
    val composers: Map<String, ComposerDraft> = emptyMap(),
    /** Last observed disk version of tracked files (open panels, drafts, config.json). */
    val disk: Map<String, DiskVersion> = emptyMap(),
    /** Effective configuration (the last valid config.json when the file is currently invalid). */
    val config: AppConfig = AppConfig(),
    /** Why config.json is not applied, with key paths; null when it is valid. */
    val configProblem: String? = null,
    val notices: List<StoreNotice> = emptyList(),
    /** Set while persisting keeps failing (for example, storage full); retried automatically. */
    val writeError: String? = null,
) {
    val activeSession: Session? get() = activeSessionId?.let(::session)?.takeIf { !it.isArchived }
    val liveSessions: List<Session> get() = sessions.filter { !it.isArchived }
    val archivedSessions: List<Session> get() = sessions.filter { it.isArchived }

    fun session(id: SessionId): Session? = sessions.firstOrNull { it.id == id }

    fun fileStatus(path: String): FileStatus = Drafts.status(drafts[path], disk[path])

    fun composer(conversationId: String): ComposerDraft = composers[conversationId] ?: ComposerDraft(conversationId)

    /** Resources with unsaved content: file drafts and non-empty composers. */
    val dirtyResources: Set<ResourceRef> get() =
        drafts.keys.map { ResourceRef.File(it) }.toSet() + composers.values.filter { it.hasContent }.map { ResourceRef.Conversation(it.conversationId) }

    fun isDirty(resource: ResourceRef): Boolean = when (resource) {
        is ResourceRef.File -> resource.path in drafts
        is ResourceRef.Conversation -> composers[resource.id]?.hasContent == true
    }

    /** Terminal ids referenced by any live session; the runtime may end the others. */
    val referencedTerminals: Set<String> get() = liveSessions.flatMap { session ->
        session.workbench.panels.values.mapNotNull { (it.target as? PanelTarget.Terminal)?.terminalId }
    }.toSet()
}

data class FileSnapshot(
    val path: String,
    /** Disk version read now (MISSING when the file does not exist). */
    val disk: DiskVersion,
    /** Disk content as text; null when missing, binary, not UTF-8 or too large. */
    val diskText: String?,
    val binary: Boolean,
    val tooLarge: Boolean,
    val draft: FileDraft?,
) {
    /** What the editor shows. */
    val text: String get() = draft?.text ?: diskText.orEmpty()
    /** The version the editor must pass to `editFile` (the draft's base when a draft exists). */
    val shownVersion: DiskVersion get() = draft?.base ?: disk
    val status: FileStatus get() = Drafts.status(draft, disk)
}

sealed interface SaveResult {
    /** Written; continue editing with [version] as the shown version. */
    data class Saved(val version: DiskVersion, val text: String) : SaveResult
    /** No draft: nothing to write. */
    data class Unchanged(val version: DiskVersion) : SaveResult
    /** The disk moved away from the draft's base; the draft is kept. */
    data class Conflict(val disk: DiskVersion) : SaveResult
    /** A validator rejected the content (config.json and similar); the draft is kept. */
    data class Invalid(val message: String) : SaveResult
    data class Failed(val message: String) : SaveResult
}

sealed interface ArchiveOutcome {
    data class Archived(val savedPaths: List<String>) : ArchiveOutcome
    /** Unsaved content is involved: ask save all / keep drafts / discard. */
    data class NeedsDecision(val resources: Set<ResourceRef>) : ArchiveOutcome
    /** Saving was requested but these files changed on disk; nothing was changed. */
    data class SaveConflict(val paths: List<String>) : ArchiveOutcome
    data class Invalid(val path: String, val message: String) : ArchiveOutcome
    data class Failed(val message: String) : ArchiveOutcome
    data object NotFound : ArchiveOutcome
}

sealed interface ConfigUpdate {
    data class Updated(val config: AppConfig) : ConfigUpdate
    /** config.json currently has errors; settings do not overwrite the user's file. */
    data class Blocked(val problem: String) : ConfigUpdate
    data class Failed(val message: String) : ConfigUpdate
}

sealed interface FileOpResult {
    data object Done : FileOpResult
    data class Failed(val message: String) : FileOpResult
}

