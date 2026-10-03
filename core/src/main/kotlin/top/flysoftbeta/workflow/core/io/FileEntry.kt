package top.flysoftbeta.workflow.core.io

/** Directory entry returned by the Engine file API. */
data class FileEntry(val path: String, val isDirectory: Boolean, val size: Long, val modifiedAt: Long) {
    val name: String get() = WorkspacePaths.name(path)
}
