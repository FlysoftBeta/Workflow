package top.flysoftbeta.workflow.ui.design.icons

import androidx.annotation.DrawableRes

/** Extension → glyph mapping for tabs, tree rows and attachment chips (docs/ux/README.md §1.3, ~12 kinds). */
object FileTypeIcons {
    private val byExtension: Map<String, Int> by lazy {
        buildMap {
            listOf("md", "markdown", "mdx").forEach { put(it, Sym.Article) }
            listOf("txt", "log", "rst", "adoc").forEach { put(it, Sym.Description) }
            listOf("kt", "kts", "java", "c", "h", "cc", "cpp", "hpp", "rs", "go", "py", "rb", "swift", "cs", "lua", "php", "gradle", "cmake")
                .forEach { put(it, Sym.Code) }
            listOf("js", "mjs", "cjs", "ts", "tsx", "jsx").forEach { put(it, Sym.Javascript) }
            listOf("html", "htm", "xml", "svg", "vue").forEach { put(it, Sym.Html) }
            listOf("css", "scss", "less").forEach { put(it, Sym.Css) }
            listOf("json", "jsonc", "yaml", "yml", "toml", "ini", "conf", "properties", "lock").forEach { put(it, Sym.DataObject) }
            listOf("png", "jpg", "jpeg", "gif", "webp", "bmp", "heic", "ico").forEach { put(it, Sym.Image) }
            put("pdf", Sym.PictureAsPdf)
            listOf("zip", "tar", "gz", "tgz", "xz", "zst", "bz2", "7z", "rar", "apk", "deb").forEach { put(it, Sym.FolderZip) }
            listOf("csv", "tsv", "xlsx", "ods").forEach { put(it, Sym.Table) }
            listOf("sh", "bash", "zsh", "fish", "ps1", "bat").forEach { put(it, Sym.Terminal) }
            listOf("mp3", "wav", "flac", "ogg", "m4a", "opus").forEach { put(it, Sym.AudioFile) }
            listOf("mp4", "mkv", "webm", "mov", "avi").forEach { put(it, Sym.VideoFile) }
        }
    }

    @DrawableRes
    fun forName(name: String, isDirectory: Boolean = false, expanded: Boolean = false): Int {
        if (isDirectory) return if (expanded) Sym.FolderOpen else Sym.Folder
        val lower = name.lowercase()
        if (lower == "dockerfile" || lower == "makefile") return Sym.Code
        val extension = lower.substringAfterLast('.', missingDelimiterValue = "")
        return byExtension[extension] ?: Sym.Draft
    }
}
