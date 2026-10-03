package top.flysoftbeta.workflow.engine.chat

import java.lang.reflect.Proxy
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.core.store.WorkspaceStore

class ConversationIndexFileTest {
    private class Documents(var body: String? = null) {
        var quarantined = false
        val store = Proxy.newProxyInstance(WorkspaceStore::class.java.classLoader, arrayOf(WorkspaceStore::class.java)) { _, method, args ->
            when (method.name) {
                "readConversationIndex" -> body
                "writeConversationIndex" -> { body = args[0] as String; Unit }
                "quarantineConversationIndex" -> { quarantined = true; body = null; Unit }
                "flush" -> true
                else -> error("Unexpected store call: ${method.name}")
            }
        } as WorkspaceStore
    }
    @Test fun refreshPreservesUserEditAwaitingEngineAcknowledgement() = runBlocking {
        val documents = Documents()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            val entered = CompletableDeferred<Unit>()
            val acknowledge = CompletableDeferred<Unit>()
            var gate = false
            val store = object : WorkspaceStore by documents.store {
                override suspend fun writeConversationIndex(text: String) {
                    if (gate) { gate = false; entered.complete(Unit); acknowledge.await() }
                    documents.store.writeConversationIndex(text)
                }
            }
            val index = ConversationIndexFile(store, scope, Dispatchers.IO).also { it.start() }
            val key = ThreadKey(BackendKind.CODEX, "thread-a")
            index.upsert(ConversationEntry("a", key.backend, key.id, null, "/workspace", 0, 0))
            gate = true
            val edit = async(start = CoroutineStart.UNDISPATCHED) { index.update("a") { it.copy(title = "User title", archived = true) } }
            entered.await()
            val refresh = async(start = CoroutineStart.UNDISPATCHED) { index.refreshThread(key, ThreadState(key, title = "Backend title"), 10) }
            acknowledge.complete(Unit)
            edit.await(); refresh.await(); index.flush()
            val written = ConversationIndexCodec.decode(documents.body!!).single()
            assertEquals("User title", written.title)
            assertTrue(written.archived)
            val reopened = ConversationIndexFile(store, scope, Dispatchers.IO).also { it.start(); it.awaitLoaded() }
            assertEquals(written, reopened.entries.value.single())
            index.remove("a")
            index.refreshThread(key, ThreadState(key), 20)
            assertTrue(index.entries.value.isEmpty())
        } finally { scope.cancel() }
    }
    @Test fun concurrentUpdatesSurviveReopen() = runBlocking {
        val documents = Documents()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            val index = ConversationIndexFile(documents.store, scope, Dispatchers.IO).also { it.start() }
            index.upsert(ConversationEntry("a", BackendKind.CODEX, null, null, "/workspace", 0, 0))
            coroutineScope {
                launch { index.update("a") { it.copy(title = "renamed") } }
                launch { index.update("a") { it.copy(archived = true) } }
                launch { index.update("a") { it.copy(model = "model", effort = "high") } }
            }
            index.flush()
            val written = ConversationIndexCodec.decode(documents.body!!).single()
            assertEquals("renamed", written.title)
            assertTrue(written.archived)
            assertEquals("model", written.model)
            assertEquals("high", written.effort)
            val reopened = ConversationIndexFile(documents.store, scope, Dispatchers.IO).also { it.start(); it.awaitLoaded() }
            assertEquals(written, reopened.entries.value.single())
        } finally { scope.cancel() }
    }
    @Test fun newerIndexIsNeverOverwritten() = runBlocking {
        val original = """{"format":99,"conversations":[]}"""
        val documents = Documents(original)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        try {
            val index = ConversationIndexFile(documents.store, scope, Dispatchers.IO).also { it.start() }
            assertTrue(runCatching { index.remove("a") }.exceptionOrNull() is ConversationIndexCodec.NewerFormatException)
            index.flush()
            assertEquals(original, documents.body)
            assertFalse(documents.quarantined)
        } finally { scope.cancel() }
    }
    @Test fun codecRetainsConversationFields() {
        val entries = listOf(ConversationEntry("a", BackendKind.CODEX, "t1", "标题", "/workspace", 1, 2, archived = true, model = "model", effort = "high", preview = "你好"), ConversationEntry("b", BackendKind.CLAUDE, null, null, "/workspace", 3, 4, forkedFrom = "a", forkedAt = "turn"))
        assertEquals(entries, ConversationIndexCodec.decode(ConversationIndexCodec.encode(entries)))
    }
}
