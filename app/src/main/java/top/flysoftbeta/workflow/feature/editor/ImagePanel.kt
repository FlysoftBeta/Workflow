package top.flysoftbeta.workflow.feature.editor

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.platform.workspace.WorkspaceIntents
import top.flysoftbeta.workflow.platform.workspace.WorkspaceResourceCache
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.RegionLoading
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import kotlinx.coroutines.CancellationException

/**
 * Image preview (docs/ui.md §4.3): fits the panel by default, pinch to zoom and drag to pan, double tap
 * switches between fit and 1:1. Decoded off the main thread, downsampled to at most 4096px, reloaded when
 * the file changes on disk.
 */
internal class ImageController(private val path: String, private val context: PanelContext) : PanelController {
    private val appContext = context.appContext
    private var bitmap by mutableStateOf<Bitmap?>(null)
    /** How many original pixels one decoded pixel covers (inSampleSize). */
    private var sample = 1
    private var failed by mutableStateOf(false)
    private var loading by mutableStateOf(true)
    /** Null = fit; else scale of decoded pixels to screen pixels. */
    private var scale by mutableStateOf<Float?>(null)
    private var offset by mutableStateOf(Offset.Zero)
    /** Fit scale of the last layout (read by gesture handlers; not snapshot state). */
    private var fitScale = 1f

    init {
        context.scope.launch {
            context.store.state.map { it.disk[path] }.distinctUntilChanged().collect { load() }
        }
        context.scope.launch { context.store.openFile(path) } // tracks the file so disk changes arrive
    }

    private suspend fun load() {
        val result = withContext(Dispatchers.IO) {
            try {
                val file = WorkspaceResourceCache.stage(appContext, path, WorkspaceResourceCache.MAX_IMAGE_BYTES)
                try {
                    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    BitmapFactory.decodeFile(file.path, bounds)
                    if (bounds.outWidth <= 0 || bounds.outHeight <= 0) null else {
                        var sampleSize = 1
                        while (maxOf(bounds.outWidth, bounds.outHeight) / sampleSize > 4096) sampleSize *= 2
                        BitmapFactory.decodeFile(file.path, BitmapFactory.Options().apply { inSampleSize = sampleSize })?.let { it to sampleSize }
                    }
                } finally { file.parentFile?.deleteRecursively() }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { null }
        }
        loading = false
        if (result == null) { failed = true; bitmap = null; return }
        failed = false
        sample = result.second
        bitmap = result.first
    }

    private fun toggleFit() {
        scale = if (scale == null) sample.toFloat() else null
        offset = Offset.Zero
    }

    override val tab: PanelTab get() = PanelTab(WorkspacePaths.name(path), Sym.Image)

    override val actions: List<ToolAction> get() = listOf(
        ToolAction("fit", Sym.FitScreen, if (scale == null) "原始大小" else "适应窗口", checked = scale != null) { toggleFit() },
    )

    override val resourceMenu: List<MenuEntry> get() = listOf(
        fileMenu(appContext, context, path)[0],
        fileMenu(appContext, context, path)[2],
        fileMenu(appContext, context, path)[3],
        MenuEntry.Action("openWith", "用其他应用打开", Sym.OpenInNew) {
            context.scope.launch {
                if (!WorkspaceIntents.openWith(appContext, path)) context.commands.snackbar("无法读取文件或没有可用的应用")
            }
        },
        MenuEntry.Action("share", "分享", Sym.Share) {
            context.scope.launch {
                if (!WorkspaceIntents.share(appContext, path)) context.commands.snackbar("无法读取文件或没有可用的应用")
            }
        },
    )

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        val colors = WorkflowTheme.colors
        val image = bitmap
        Box(modifier.fillMaxSize().background(colors.surface).testTag("image:$path"), contentAlignment = Alignment.Center) {
            when {
                image != null -> ZoomableImage(image)
                failed -> EmptyState(listOf(TextAction("用其他应用打开") {
                    context.scope.launch {
                        if (!WorkspaceIntents.openWith(appContext, path)) context.commands.snackbar("无法读取文件或没有可用的应用")
                    }
                }), message = "无法预览此图片")
                else -> RegionLoading(loading, Modifier.fillMaxSize())
            }
        }
    }

    @Composable
    private fun ZoomableImage(image: Bitmap) {
        val density = LocalDensity.current
        BoxWithConstraints(Modifier.fillMaxSize().clipToBounds(), contentAlignment = Alignment.Center) {
            val boxW = constraints.maxWidth.toFloat()
            val boxH = constraints.maxHeight.toFloat()
            fitScale = minOf(boxW / image.width, boxH / image.height).coerceAtMost(sample.toFloat().coerceAtLeast(1f) * 4f)
            val current = scale ?: fitScale
            Image(
                image.asImageBitmap(), contentDescription = WorkspacePaths.name(path),
                contentScale = ContentScale.FillBounds,
                modifier = Modifier
                    .requiredSize(with(density) { image.width.toDp() }, with(density) { image.height.toDp() })
                    .graphicsLayer {
                        scaleX = current; scaleY = current
                        translationX = offset.x; translationY = offset.y
                    },
            )
            Box(Modifier.fillMaxSize()
                .pointerInput(image) {
                    detectTapGestures(onDoubleTap = { toggleFit() })
                }
                .pointerInput(image) {
                    detectTransformGestures { _, pan, zoom, _ ->
                        val next = ((scale ?: fitScale) * zoom).coerceIn(fitScale.coerceAtMost(1f) * 0.5f, sample * 8f)
                        scale = next
                        offset += pan
                    }
                })
        }
    }

    override fun dispose() { bitmap = null }
}
