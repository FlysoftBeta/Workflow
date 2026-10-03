package top.flysoftbeta.workflow.platform.workspace

import java.io.File
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class BoundedResourceCacheTest {
    @get:Rule val temporary = TemporaryFolder()

    @Test fun streamsChunksIntoTransientCopyAndRetainsOriginalName() = runBlocking<Unit> {
        val bytes = ByteArray(140000) { (it % 127).toByte() }
        var calls = 0
        val result = BoundedResourceCache(temporary.root).stage("pictures/test.png", 150000) { offset, length ->
            calls++
            val next = minOf(offset.toInt() + length, bytes.size)
            ResourceChunk(bytes.copyOfRange(offset.toInt(), next), next.toLong(), next == bytes.size, bytes.size.toLong())
        }
        assertEquals(3, calls)
        assertEquals("test.png", result.name)
        assertArrayEquals(bytes, result.readBytes())
    }

    @Test fun rejectsOverLimitBeforeWritingAndDeletesPartialCopy() = runBlocking<Unit> {
        val result = runCatching { BoundedResourceCache(temporary.root).stage("large.bin", 100) { _, _ ->
            ResourceChunk(byteArrayOf(1), 1, false, 101)
        } }
        assertTrue(result.isFailure)
        assertTrue(temporary.root.listFiles()!!.isEmpty())
    }

    @Test fun rejectsChangedFileAndInvalidOffsetsWithoutExposingPartialCopy() = runBlocking<Unit> {
        val result = runCatching { BoundedResourceCache(temporary.root).stage("changed.bin", 100) { offset, _ ->
            if (offset == 0L) ResourceChunk(byteArrayOf(1), 1, false, 3)
            else ResourceChunk(byteArrayOf(2), 2, false, 4)
        } }
        assertTrue(result.isFailure)
        val invalid = runCatching { BoundedResourceCache(temporary.root).stage("invalid.bin", 100) { _, _ ->
            ResourceChunk(byteArrayOf(1), 7, true, 7)
        } }
        assertTrue(invalid.isFailure)
        assertTrue(temporary.root.listFiles()!!.isEmpty())
    }

    @Test fun cancellationDeletesPartialCopy() = runBlocking<Unit> {
        val result = runCatching { BoundedResourceCache(temporary.root).stage("cancel.bin", 100) { _, _ ->
            throw CancellationException("cancel")
        } }
        assertTrue(result.exceptionOrNull() is CancellationException)
        assertTrue(temporary.root.listFiles()!!.isEmpty())
    }

    @Test fun forbidsEscapeAndPrivateWorkspaceStateBeforeReading() = runBlocking<Unit> {
        listOf("../outside", "/absolute", ".workspace/state/private.json").forEach { path ->
            val result = runCatching { BoundedResourceCache(temporary.root).stage(path, 100) { _, _ ->
                fail("must not read $path"); error("unexpected")
            } }
            assertTrue(result.isFailure)
        }
    }

    @Test fun budgetProtectsRecentSharesAndReclaimsExpiredCopies() = runBlocking<Unit> {
        var clock = 1_000_000L
        val cache = BoundedResourceCache(temporary.root, budget = 10, now = { clock })
        val source: suspend (Long, Int) -> ResourceChunk = { _, _ -> ResourceChunk(ByteArray(6), 6, true, 6) }
        val first = cache.stage("first.txt", 10, source)
        assertTrue(runCatching { cache.stage("second.txt", 10, source) }.isFailure)
        assertTrue(first.exists())
        clock += 3_600_001L
        assertTrue(cache.stage("second.txt", 10, source).exists())
        assertFalse(first.exists())
    }
}
