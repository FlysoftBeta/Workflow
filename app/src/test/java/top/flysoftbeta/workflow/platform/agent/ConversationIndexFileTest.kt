package top.flysoftbeta.workflow.platform.agent

import top.flysoftbeta.workflow.core.store.ReferenceWorkspaceStore

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.ConversationEntry
import top.flysoftbeta.workflow.core.io.MemoryFileSystem
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.io.writeAtomic
import top.flysoftbeta.workflow.core.store.WorkspaceStore

class ConversationIndexFileTest {
    private val path = ".workspace/state/conversations.json"

    @Test fun backgroundRefreshPreservesUserEditAwaitingEngineAcknowledgement() = runBlocking {
        val fs = MemoryFileSystem()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            val backing = ReferenceWorkspaceStore(fs, scope, Dispatchers.IO).also { it.start(); it.awaitReady() }
            val entered = CompletableDeferred<Unit>()
            val acknowledge = CompletableDeferred<Unit>()
            var gate = false
            val store = object : WorkspaceStore by backing {
                override suspend fun writeConversationIndex(text: String) {
                    if (gate) { gate = false; entered.complete(Unit); acknowledge.await() }
                    backing.writeConversationIndex(text)
                }
            }
            val index = ConversationIndexFile(store, scope, Dispatchers.IO).also { it.start() }
            val key = top.flysoftbeta.workflow.agent.model.ThreadKey(BackendKind.CODEX, "thread-a")
            index.upsert(ConversationEntry("a", key.backend, key.id, null, "/workspace", 0, 0))
            gate = true
            val edit = async(start = CoroutineStart.UNDISPATCHED) { index.update("a") { it.copy(title = "User title", archived = true) } }
            entered.await()
            val refresh = async(start = CoroutineStart.UNDISPATCHED) {
                index.refreshThread(key, top.flysoftbeta.workflow.agent.model.ThreadState(key, title = "Backend title"), 10)
            }
            acknowledge.complete(Unit)
            edit.await(); refresh.await(); index.flush()
            val written = ConversationIndexCodec.decode(fs.readText(path)).single()
            assertEquals("User title", written.title)
            assertTrue(written.archived)
            val reopened = ConversationIndexFile(store, scope, Dispatchers.IO).also { it.start(); it.awaitLoaded() }
            assertEquals(written, reopened.entries.value.single())
            index.remove("a")
            index.refreshThread(key, top.flysoftbeta.workflow.agent.model.ThreadState(key), 20)
            assertTrue(index.entries.value.isEmpty())
        } finally { scope.cancel() }
    }

    @Test fun concurrentUpdatesSurviveStoreFlushAndReopen() = runBlocking {
        val fs = MemoryFileSystem()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            val store = ReferenceWorkspaceStore(fs, scope, Dispatchers.IO).also { it.start(); it.awaitReady() }
            val index = ConversationIndexFile(store, scope, Dispatchers.IO).also { it.start() }
            index.upsert(ConversationEntry("a", BackendKind.CODEX, null, null, "/workspace", 0, 0))
            coroutineScope {
                launch { index.update("a") { it.copy(title = "renamed") } }
                launch { index.update("a") { it.copy(archived = true) } }
                launch { index.update("a") { it.copy(model = "model", effort = "high") } }
            }
            index.flush()
            val entry = ConversationIndexCodec.decode(fs.readText(path)).single()
            assertEquals("renamed", entry.title)
            assertTrue(entry.archived)
            assertEquals("model", entry.model)
            assertEquals("high", entry.effort)
            val reopened = ConversationIndexFile(store, scope, Dispatchers.IO).also { it.start(); it.awaitLoaded() }
            assertEquals(listOf(entry), reopened.entries.value)
        } finally { scope.cancel() }
    }

    @Test fun newerIndexIsNeverOverwritten() = runBlocking {
        val fs = MemoryFileSystem()
        val original = """{"format":99,"conversations":[]}"""
        fs.writeAtomic(path, original)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            val store = ReferenceWorkspaceStore(fs, scope, Dispatchers.IO).also { it.start(); it.awaitReady() }
            val index = ConversationIndexFile(store, scope, Dispatchers.IO).also { it.start() }
            val error = runCatching { index.remove("a") }.exceptionOrNull()
            assertTrue(error is ConversationIndexCodec.NewerFormatException)
            index.flush()
            assertEquals(original, fs.readText(path))
            assertTrue(fs.list(".workspace/state/corrupt").isEmpty())
        } finally { scope.cancel() }
    }
}
