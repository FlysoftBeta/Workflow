package top.flysoftbeta.workflow.client.sync

import top.flysoftbeta.workflow.core.connection.WorkspaceRpc

/** Shared client document sync; callers never persist a competing Android source of truth. */
class WorkspaceDocuments(rpc: WorkspaceRpc) {
    private val sync = RevisionedDocumentSync(RpcDocuments(rpc))
    suspend fun read(namespace: String, key: String): RevisionedDocument = sync.read(DocumentKey(namespace, key))
    suspend fun write(namespace: String, key: String, text: String, expectedRevision: Long) {
        sync.publishProduced(DocumentKey(namespace, key), text, expectedRevision)
    }
}
