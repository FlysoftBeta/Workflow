package top.flysoftbeta.workflow.platform.importer

import top.flysoftbeta.workflow.core.store.ReferenceWorkspaceStore

import androidx.core.content.FileProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import top.flysoftbeta.workflow.core.io.MemoryFileSystem
import top.flysoftbeta.workflow.core.io.readText
import top.flysoftbeta.workflow.core.store.WorkspaceStore

/** Content-URI import through the real ContentResolver into an in-memory workspace: names never collide. */
@RunWith(AndroidJUnit4::class)
class ImportServiceTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val fs = MemoryFileSystem(System::currentTimeMillis)
    private val store = ReferenceWorkspaceStore(fs, scope, Dispatchers.IO.limitedParallelism(1)).also { it.start() }
    private val captures = File(context.cacheDir, "captures").apply { mkdirs() }
    private val source = File(captures, "photo ${UUID.randomUUID().toString().take(4)}.txt").apply { writeText("pixels") }

    @After fun tearDown() {
        source.delete()
        scope.cancel()
    }

    @Test fun importsProviderContentUnderFreeNamesAndCleansStaging() = runBlocking {
        store.awaitReady()
        val service = ImportService(context) { store }
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", source)
        val first = service.importUris(listOf(uri, uri), "inbox") as ImportResult.Imported
        val second = service.importUris(listOf(uri), "inbox") as ImportResult.Imported
        val name = source.name.removeSuffix(".txt")
        assertEquals(listOf("inbox/$name.txt", "inbox/$name (1).txt"), first.paths)
        assertEquals(listOf("inbox/$name (2).txt"), second.paths)
        assertTrue(first.paths.all { fs.readText(it) == "pixels" })
        assertTrue(File(context.cacheDir, "import-staging").listFiles().orEmpty().isEmpty())
        // Unreadable content is reported, not silently dropped.
        val missing = service.importUris(listOf(android.net.Uri.parse("content://${context.packageName}.files/captures/none.bin")), "inbox")
        assertTrue(missing is ImportResult.Failed)
    }
}
