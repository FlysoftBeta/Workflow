package top.flysoftbeta.workflow.client.sync

import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.client.protocol.*

class ConfigurationSyncTest {
    @Test fun conflictReadsFreshAuthorityAndReappliesOnlyTheIntendedTransformation() = runBlocking {
        var document = RevisionedDocument("remote", 4)
        val writes = mutableListOf<Pair<String, Long>>()
        val remote = object : DocumentRemote {
            override suspend fun read(key: DocumentKey) = document
            override suspend fun write(key: DocumentKey, text: String, expectedRevision: Long): Long {
                writes += text to expectedRevision
                if (writes.size == 1) { document = RevisionedDocument("remote+other", 5); throw ConfigurationConflict() }
                assertEquals(document.revision, expectedRevision)
                document = RevisionedDocument(text, expectedRevision + 1)
                return document.revision
            }
        }
        val result = RevisionedDocumentSync(remote).transform(DocumentKey("test", "settings")) { "$it+user" }
        assertEquals(listOf("remote+user" to 4L, "remote+other+user" to 5L), writes)
        assertEquals(RevisionedDocument("remote+other+user", 6), result)
    }
    @Test fun producedDataConflictIsNotRetriedOrOverwritten() = runBlocking {
        var writes = 0
        val remote = object : DocumentRemote {
            override suspend fun read(key: DocumentKey) = error("Produced data must keep its original base")
            override suspend fun write(key: DocumentKey, text: String, expectedRevision: Long): Long { writes++; throw ConfigurationConflict() }
        }
        assertTrue(runCatching { RevisionedDocumentSync(remote).publishProduced(DocumentKey("services.proxy", "runtime.log"), "redacted", 9) }.exceptionOrNull() is ConfigurationConflict)
        assertEquals(1, writes)
    }
    private fun config(revision: Int, theme: String) = ClientConfigReply.decode(parseWire("""{"revision":$revision,"config":{"appearance":{"theme":"$theme"},"overlay":{},"launcher":["workbench","proxy","settings"],"terminal":{}}}"""))
    @Test fun reconnectDiscardsRevisionAndCannotReuseAnotherWorkspaceProjection() = runBlocking {
        val a = ClientConfigurationSync(WorkspaceIdentity("profile-a", "/a")) { config(30, "dark") }
        val b = ClientConfigurationSync(WorkspaceIdentity("profile-b", "/b")) { config(1, "light") }
        assertEquals("dark", a.refresh().configuration.appearance.theme)
        assertNull(b.state.value)
        assertEquals("light", b.refresh().configuration.appearance.theme)
        assertNotEquals(a.state.value!!.identity, b.state.value!!.identity)
    }
    @Test fun sameConnectionRejectsRegressingSnapshots() = runBlocking {
        var revision = 8
        val sync = ClientConfigurationSync(WorkspaceIdentity("profile", "/workspace")) { config(revision--, "system") }
        sync.refresh()
        assertTrue(runCatching { sync.refresh() }.isFailure)
        assertEquals(8L, sync.state.value!!.revision)
    }
}
