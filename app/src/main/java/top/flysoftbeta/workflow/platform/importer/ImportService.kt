package top.flysoftbeta.workflow.platform.importer

import android.content.ActivityNotFoundException
import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.FileProvider
import java.io.File
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.core.io.FileNames
import top.flysoftbeta.workflow.core.io.WorkspacePaths
import top.flysoftbeta.workflow.core.store.FileOpResult
import top.flysoftbeta.workflow.core.store.WorkspaceStore

/** Where imported content comes from (the explorer's `＋` and the composer's `+`). */
enum class ImportKind {
    /** A new photo from the camera app (ACTION_IMAGE_CAPTURE into a FileProvider URI). */
    CAMERA,
    /** Images from a gallery app (ACTION_GET_CONTENT for images, multiple). */
    GALLERY,
    /** Documents of any type (ACTION_OPEN_DOCUMENT, multiple). */
    FILES,
}

/**
 * One import. [targetDir] is a workspace-relative directory ("" = root; created when missing).
 * [owner] is an opaque tag of the caller ("conversation:<id>" for chat attachments, "explorer") that comes
 * back with [ImportService.orphaned] results, so a result that arrives after process death can still be
 * routed.
 */
data class ImportRequest(val kind: ImportKind, val targetDir: String, val owner: String? = null)

sealed interface ImportResult {
    /** Workspace paths of the new files, in pick order. [failures] names items that could not be copied. */
    data class Imported(val paths: List<String>, val failures: List<String> = emptyList()) : ImportResult
    data object Cancelled : ImportResult
    data class Failed(val message: String) : ImportResult
}

/**
 * The shared importer (product.md §7/§8): camera, gallery and device files into a workspace folder, plus
 * content dropped from other apps. One process-wide instance; the activity attaches its result launchers
 * in `onCreate` ([attach]). Content is staged in the app cache off the main thread (slow providers never
 * hold the workspace store), then committed with [WorkspaceStore.importFile] under a free name
 * (`name (n).ext`), never overwriting.
 */
class ImportService internal constructor(context: Context, private val storeProvider: () -> WorkspaceStore) {
    private val appContext = context.applicationContext
    private val store: WorkspaceStore get() = storeProvider()

    private class Launchers(
        val camera: ActivityResultLauncher<Uri>,
        val gallery: ActivityResultLauncher<String>,
        val files: ActivityResultLauncher<Array<String>>,
    )

    private class Pending(val request: ImportRequest, val capture: File?, val result: CompletableDeferred<ImportResult>?)

    private var launchers: Launchers? = null
    private var pending: Pending? = null
    private val orphanResults = MutableSharedFlow<Pair<ImportRequest, ImportResult>>(extraBufferCapacity = 8)

    /**
     * Results of picks whose caller is gone (the process was recreated while the picker was open). The
     * explorer shows them as a snackbar; the chat workstream attaches `conversation:*` owners.
     */
    val orphaned: SharedFlow<Pair<ImportRequest, ImportResult>> = orphanResults.asSharedFlow()

    /** Registers the result launchers; call from `Activity.onCreate` (before it is started). Main thread. */
    fun attach(activity: ComponentActivity) {
        val registry = activity.activityResultRegistry
        activity.savedStateRegistry.registerSavedStateProvider(STATE_KEY) { saveState() }
        activity.savedStateRegistry.consumeRestoredStateForKey(STATE_KEY)?.let { restored ->
            if (pending == null) pending = restoreState(restored)
        }
        val registered = Launchers(
            camera = registry.register("$STATE_KEY.camera", activity, ActivityResultContracts.TakePicture()) { saved ->
                complete { request, capture -> if (saved && capture != null) importFiles(listOf(capture), request.targetDir, cameraName()) else ImportResult.Cancelled }
            },
            gallery = registry.register("$STATE_KEY.gallery", activity, ActivityResultContracts.GetMultipleContents()) { uris ->
                complete { request, _ -> if (uris.isEmpty()) ImportResult.Cancelled else importUris(uris, request.targetDir) }
            },
            files = registry.register("$STATE_KEY.files", activity, ActivityResultContracts.OpenMultipleDocuments()) { uris ->
                complete { request, _ -> if (uris.isEmpty()) ImportResult.Cancelled else importUris(uris, request.targetDir) }
            },
        )
        launchers = registered
        activity.lifecycle.addObserver(object : androidx.lifecycle.DefaultLifecycleObserver {
            override fun onDestroy(owner: androidx.lifecycle.LifecycleOwner) {
                // The registry unregisters with the activity; never keep a dead activity's launchers.
                if (launchers === registered) launchers = null
            }
        })
    }

    /**
     * Opens the picker for [kind] and imports what the user chose into [targetDir]. Main thread. A second
     * pick while one is open cancels the first.
     */
    suspend fun pick(kind: ImportKind, targetDir: String, owner: String? = null): ImportResult {
        val directory = WorkspacePaths.normalizeOrNull(targetDir)?.takeIf { !WorkspacePaths.isReserved(it) || it.isEmpty() }
            ?: return ImportResult.Failed("目标文件夹无效")
        val active = launchers ?: return ImportResult.Failed("无法打开选择器")
        pending?.result?.complete(ImportResult.Cancelled)
        val result = CompletableDeferred<ImportResult>()
        val request = ImportRequest(kind, directory, owner)
        try {
            when (kind) {
                ImportKind.CAMERA -> {
                    val capture = withContext(Dispatchers.IO) {
                        File(appContext.cacheDir, "captures").apply { mkdirs() }.let { File(it, "${UUID.randomUUID()}.jpg") }
                    }
                    pending = Pending(request, capture, result)
                    active.camera.launch(FileProvider.getUriForFile(appContext, "${appContext.packageName}.files", capture))
                }
                ImportKind.GALLERY -> { pending = Pending(request, null, result); active.gallery.launch("image/*") }
                ImportKind.FILES -> { pending = Pending(request, null, result); active.files.launch(arrayOf("*/*")) }
            }
        } catch (_: ActivityNotFoundException) {
            pending = null
            return ImportResult.Failed(if (kind == ImportKind.CAMERA) "没有可用的相机应用" else "没有可用的文件选择器")
        }
        return try { result.await() } finally { if (pending?.result === result) pending = null }
    }

    /** Imports content URIs (drag and drop from another app, or picker results) into [targetDir]. */
    suspend fun importUris(uris: List<Uri>, targetDir: String): ImportResult {
        val directory = WorkspacePaths.normalizeOrNull(targetDir) ?: return ImportResult.Failed("目标文件夹无效")
        val paths = ArrayList<String>()
        val failures = ArrayList<String>()
        for (uri in uris) {
            var label = uri.lastPathSegment ?: "文件"
            try {
                val staged = withContext(Dispatchers.IO) {
                    val resolver = appContext.contentResolver
                    val display = if (uri.scheme == "file") uri.lastPathSegment else runCatching {
                        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                            if (c.moveToFirst() && !c.isNull(0)) c.getString(0) else null
                        }
                    }.getOrNull()
                    val extension = resolver.getType(uri)?.let { android.webkit.MimeTypeMap.getSingleton().getExtensionFromMimeType(it) }
                    val fallback = "导入-${stamp()}" + (extension?.let { ".$it" } ?: "")
                    label = FileNames.sanitize(display, fallback)
                    val file = stagingFile()
                    try {
                        (resolver.openInputStream(uri) ?: throw IOException("无法读取")).use { input ->
                            file.outputStream().use { input.copyTo(it, 1 shl 16) }
                        }
                    } catch (error: Exception) {
                        file.delete(); throw error
                    }
                    file to label
                }
                try { paths += commit(staged.first, directory, staged.second) } finally { withContext(Dispatchers.IO) { staged.first.delete() } }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                failures += label
            }
        }
        return if (paths.isEmpty() && failures.isNotEmpty()) ImportResult.Failed("无法导入 ${failures.joinToString("、")}")
        else ImportResult.Imported(paths, failures)
    }

    private suspend fun importFiles(files: List<File>, targetDir: String, name: String): ImportResult {
        val paths = ArrayList<String>()
        try {
            files.forEach { paths += commit(it, targetDir, name) }
        } catch (error: IOException) {
            return ImportResult.Failed(error.message ?: "导入失败")
        } finally {
            withContext(Dispatchers.IO) { files.forEach { it.delete() } }
        }
        return ImportResult.Imported(paths)
    }

    /** Moves a staged file into [directory] under the first free variant of [name]. */
    private suspend fun commit(staged: File, directory: String, name: String): String {
        val store = kotlinx.coroutines.withTimeout(45_000) {
            var connected: WorkspaceStore? = null
            while (connected == null) {
                connected = runCatching { storeProvider() }.getOrNull()
                if (connected == null) kotlinx.coroutines.delay(100)
            }
            connected
        }
        if (directory.isNotEmpty()) store.createDirectory(directory) // "Already exists" is fine
        val taken = store.listDirectory(directory, showHidden = true).map { it.name }.toSet()
        for (candidate in FileNames.variants(name).filter { it !in taken }.take(MAX_ATTEMPTS)) {
            val path = WorkspacePaths.child(directory, candidate)
            when (val result = store.importFile(path, staged.length()) { staged.inputStream() }) {
                FileOpResult.Done -> return path
                is FileOpResult.Failed -> if (!result.message.startsWith("Already exists")) throw IOException(result.message)
            }
        }
        throw IOException("没有可用的文件名")
    }

    /** Delivers a picker result: to the waiting caller, else (after process death) to [orphaned]. */
    private fun complete(block: suspend (ImportRequest, File?) -> ImportResult) {
        val current = pending ?: return
        pending = null
        AppGraph.processScope.launch(Dispatchers.Main.immediate) {
            val result = try { block(current.request, current.capture) } catch (cancelled: CancellationException) { throw cancelled }
                catch (error: Exception) { ImportResult.Failed(error.message ?: "导入失败") }
            val waiting = current.result
            if (waiting != null && waiting.isActive) waiting.complete(result)
            else if (result !is ImportResult.Cancelled) orphanResults.emit(current.request to result)
        }
    }

    private fun saveState(): Bundle = Bundle().apply {
        pending?.let { p ->
            putString("kind", p.request.kind.name)
            putString("targetDir", p.request.targetDir)
            putString("owner", p.request.owner)
            putString("capture", p.capture?.path)
        }
    }

    private fun restoreState(state: Bundle): Pending? {
        val kind = state.getString("kind")?.let { name -> ImportKind.entries.firstOrNull { it.name == name } } ?: return null
        val targetDir = state.getString("targetDir") ?: return null
        return Pending(ImportRequest(kind, targetDir, state.getString("owner")), state.getString("capture")?.let(::File), null)
    }

    private fun stagingFile(): File = File(appContext.cacheDir, "import-staging").apply { mkdirs() }.let { File(it, UUID.randomUUID().toString()) }

    private fun cameraName() = "IMG_${stamp()}.jpg"

    private fun stamp(): String = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.ROOT).format(Date())

    companion object {
        private const val STATE_KEY = "workflow.importer"
        private const val MAX_ATTEMPTS = 50

        @Volatile private var instance: ImportService? = null

        fun get(context: Context): ImportService = instance ?: synchronized(this) {
            instance ?: context.applicationContext.let { app -> ImportService(app) { AppGraph.workspaceStore(app) } }.also { instance = it }
        }
    }
}
