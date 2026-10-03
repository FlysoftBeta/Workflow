package top.flysoftbeta.workflow.core.store

import java.io.InputStream
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.StateFlow
import top.flysoftbeta.workflow.core.config.AppConfig
import top.flysoftbeta.workflow.core.io.FileEntry
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.resource.ConflictResolution
import top.flysoftbeta.workflow.core.resource.DiskVersion
import top.flysoftbeta.workflow.core.session.ArchiveDecision
import top.flysoftbeta.workflow.core.session.SessionId

/**
 * Client projection and command port of the authoritative Rust Workspace Engine.
 * Production implementations have no local workspace writer or fallback. The test-only reference
 * implementation is published solely through Gradle test fixtures for differential verification.
 */
interface WorkspaceStore {
    val state: StateFlow<WorkspaceState>
    fun start()
    suspend fun awaitReady(): WorkspaceState
    suspend fun flush(): Boolean
    suspend fun close()
    suspend fun enterWorkbench(): SessionId
    suspend fun createSession(name: String? = null): SessionId
    suspend fun openInSeparateSession(target: PanelTarget): SessionId
    suspend fun activateSession(id: SessionId): Boolean
    suspend fun renameSession(id: SessionId, name: String): Boolean
    suspend fun restoreSession(id: SessionId): Boolean
    suspend fun archiveSession(id: SessionId, decision: ArchiveDecision? = null): ArchiveOutcome
    suspend fun pinSession(id: SessionId, index: Int): Boolean
    suspend fun unpinSession(id: SessionId): Boolean
    suspend fun runMaintenance()
    suspend fun dismissNotice(id: String)
    fun layout(sessionId: SessionId, op: LayoutOp)
    fun layout(op: LayoutOp)
    suspend fun applyLayout(sessionId: SessionId, op: LayoutOp): Workbench?
    suspend fun openFile(path: String): FileSnapshot
    fun editFile(path: String, text: String, shown: DiskVersion)
    suspend fun saveFile(path: String, text: String? = null): SaveResult
    suspend fun resolveConflict(path: String, resolution: ConflictResolution)
    suspend fun discardDraft(path: String)
    suspend fun movePath(from: String, to: String): FileOpResult
    suspend fun deletePath(path: String): FileOpResult
    suspend fun trashPath(path: String): TrashResult
    suspend fun restoreFromTrash(id: String): RestoreResult
    suspend fun purgeTrash(): Int
    suspend fun copyPath(from: String, to: String): FileOpResult
    suspend fun createFile(path: String, bytes: ByteArray = ByteArray(0)): FileOpResult
    /** Streams exactly [size] bytes to an Engine-owned staging upload. Opens and closes the source on IO. */
    suspend fun importFile(path: String, size: Long, open: () -> InputStream): FileOpResult
    /** Engine chooses a free destination atomically; clients never enumerate names to allocate. */
    suspend fun importUnique(directory: String, name: String, size: Long, open: () -> InputStream): String
    suspend fun createDirectory(path: String): FileOpResult
    suspend fun listDirectory(path: String, showHidden: Boolean = false): List<FileEntry>
    fun directoryChanges(path: String): Flow<Unit>
    suspend fun editComposer(draft: ComposerDraft, expectedRevision: Long): ComposerDraft
    suspend fun acknowledgeComposer(submitted: ComposerDraft): ComposerDraft
    suspend fun discardComposer(conversationId: String): ComposerDraft
    suspend fun removeConversation(conversationId: String)
    suspend fun updateConfig(transform: (AppConfig) -> AppConfig): ConfigUpdate
    suspend fun readConversationIndex(): String?
    suspend fun writeConversationIndex(text: String)
    suspend fun quarantineConversationIndex()
}
