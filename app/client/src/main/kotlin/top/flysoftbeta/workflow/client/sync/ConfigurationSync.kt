package top.flysoftbeta.workflow.client.sync

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.client.protocol.*
import top.flysoftbeta.workflow.core.connection.WorkspaceRpc
import top.flysoftbeta.workflow.core.connection.WorkspaceRpcException
import top.flysoftbeta.workflow.core.json.Json as LegacyJson

/** Cache keys bind presentation to the selected connection and the root returned by its hello. */
data class WorkspaceIdentity(val connectionId: String, val workspaceRoot: String)
data class ConfigurationSnapshot(val identity: WorkspaceIdentity, val revision: Long, val configuration: ClientConfiguration)

/** One instance per live connection. Reconnect discards the projection and loads Engine authority. */
class ClientConfigurationSync(
    private val identity: WorkspaceIdentity,
    private val fetch: suspend () -> ClientConfigReply,
) {
    constructor(rpc: WorkspaceRpc, identity: WorkspaceIdentity) : this(identity, {
        rpc.request(EngineMethods.client_config, ClientConfigParams.decode(buildJsonObject { put("clientId", identity.connectionId) }))
    })
    private val reads = Mutex()
    private val current = MutableStateFlow<ConfigurationSnapshot?>(null)
    val state = current.asStateFlow()
    suspend fun refresh(): ConfigurationSnapshot = reads.withLock {
        val reply = fetch()
        val revision = reply.revision.longValueExact().also { require(it >= 0) }
        val previous = current.value
        check(previous == null || revision >= previous.revision) { "Engine configuration revision regressed within a connection" }
        ConfigurationSnapshot(identity, revision, reply.config).also { current.value = it }
    }
}

data class DocumentKey(val namespace: String, val key: String) {
    init { require(listOf(namespace, key).all { it.length in 1..160 && it != "." && it != ".." && ID.matches(it) }) }
    companion object { private val ID = Regex("[A-Za-z0-9_.-]+") }
}
data class RevisionedDocument(val text: String?, val revision: Long)
class ConfigurationConflict : IllegalStateException("Configuration changed in the workspace")

/** A document port is testable without a socket or an Android cache. */
interface DocumentRemote {
    suspend fun read(key: DocumentKey): RevisionedDocument
    suspend fun write(key: DocumentKey, text: String, expectedRevision: Long): Long
}

class RpcDocuments(private val rpc: WorkspaceRpc) : DocumentRemote {
    private fun params(key: DocumentKey, text: String? = null, revision: Long? = null) = DocumentParams.decode(buildJsonObject {
        put("namespace", key.namespace); put("key", key.key)
        if (text != null) put("document", text)
        if (revision != null) put("expectedRevision", revision)
    })
    override suspend fun read(key: DocumentKey): RevisionedDocument = rpc.request(EngineMethods.documents_read, params(key)).let {
        RevisionedDocument(it.document, it.revision.longValueExact())
    }
    override suspend fun write(key: DocumentKey, text: String, expectedRevision: Long): Long {
        require(expectedRevision >= 0)
        return try { rpc.request(EngineMethods.documents_write, params(key, text, expectedRevision)).revision.longValueExact() }
        catch (error: WorkspaceRpcException) {
            val detail = error.data?.let { ErrorData.decode(parseWire(LegacyJson.stringify(it))) }
            if (detail?.kind == "conflict") throw ConfigurationConflict()
            throw error
        }
    }
}

/** Workspace is authoritative. Retry transformations against fresh data; never replay stale text. */
class RevisionedDocumentSync(private val remote: DocumentRemote) {
    private val writes = Mutex()
    suspend fun read(key: DocumentKey) = remote.read(key)
    suspend fun transform(key: DocumentKey, edit: (String?) -> String): RevisionedDocument = writes.withLock {
        repeat(5) {
            val before = remote.read(key)
            val next = edit(before.text)
            if (next == before.text) return@withLock before
            try { return@withLock RevisionedDocument(next, remote.write(key, next, before.revision)) }
            catch (_: ConfigurationConflict) { /* Read fresh authority and reapply the caller's edit. */ }
        }
        throw ConfigurationConflict()
    }
    /** Produced data has an exact base revision. Conflicts are surfaced, never overwritten or replayed. */
    suspend fun publishProduced(key: DocumentKey, text: String, expectedRevision: Long): RevisionedDocument = writes.withLock {
        RevisionedDocument(text, remote.write(key, text, expectedRevision))
    }
}
