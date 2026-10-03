package top.flysoftbeta.workflow.ui.design.dnd

import android.net.Uri
import androidx.annotation.DrawableRes
import androidx.compose.runtime.Immutable
import top.flysoftbeta.workflow.ui.design.icons.Sym

/**
 * What is being dragged. [label] and [icon] feed the drag shadow chip. Payloads carry opaque ids only;
 * feature code maps them to its model when the single drop callback fires.
 */
interface DragPayload {
    val label: String
    @get:DrawableRes val icon: Int
}

/** A tab / panel being moved between or within Stacks. */
@Immutable
data class PanelDragPayload(
    val panelId: String,
    val sourceStackId: String,
    val sourceIndex: Int,
    override val label: String,
    override val icon: Int,
) : DragPayload

/** One or more workspace-relative paths (file tree nodes, editor tabs as files). */
@Immutable
data class FilesDragPayload(
    val paths: List<String>,
    override val label: String = paths.singleOrNull()?.substringAfterLast('/') ?: "${paths.size} 项",
    override val icon: Int = if (paths.size == 1) Sym.Draft else Sym.Folder,
) : DragPayload

/** A Launcher / overlay app entry being reordered. */
@Immutable
data class AppDragPayload(
    val appId: String,
    val sourceIndex: Int,
    override val label: String,
    override val icon: Int = Sym.Apps,
) : DragPayload

/**
 * Content dragged in from another app through the platform DnD (split-screen on Android 9).
 * [uris] is empty while hovering (the platform only reveals ClipData on drop); read permissions are
 * requested before the drop callback runs.
 */
@Immutable
data class ExternalDragPayload(
    val uris: List<Uri>,
    val mimeTypes: Set<String>,
    override val label: String = if (uris.size > 1) "${uris.size} 个文件" else "文件",
    override val icon: Int = Sym.UploadFile,
) : DragPayload
