package top.flysoftbeta.workflow.platform.clientservices

import top.flysoftbeta.workflow.core.connection.WorkspaceRpc

/** Opaque service preferences live in Engine documents, never in a second Android repository. */
class WorkspaceDocuments(private val rpc: WorkspaceRpc) {
    data class Document(val text: String?, val revision: Long)

    suspend fun read(namespace: String, key: String): Document {
        val value = WorkspaceRpc.obj(rpc.request("documents.read", parameters(namespace, key)))
        return Document(value["document"] as? String,
            (value["revision"] as? Number)?.toLong() ?: error("工作区文档没有版本"))
    }

    suspend fun write(namespace: String, key: String, text: String, expectedRevision: Long) {
        rpc.request("documents.write", parameters(namespace, key) + mapOf("document" to text, "expectedRevision" to expectedRevision))
    }

    private fun parameters(namespace: String, key: String): Map<String, Any?> {
        require(listOf(namespace, key).all { it.length in 1..160 && it != "." && it != ".." && ID.matches(it) })
        return mapOf("namespace" to namespace, "key" to key)
    }

    companion object { private val ID = Regex("[A-Za-z0-9_.-]+") }
}
