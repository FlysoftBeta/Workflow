package top.flysoftbeta.workflow.core.terminal

import java.nio.ByteBuffer
import java.nio.CharBuffer
import java.nio.charset.CharsetDecoder
import java.nio.charset.CoderResult
import org.junit.Assert.*
import org.junit.Test

class StreamingUtf8DecoderTest {
    @Test fun everyBoundaryPreservesUnicodeAndTerminalControlBytes() {
        val expected = "prompt> \u001b[32m🚀测试 🌟中文\r\n\u001b[0m"
        val bytes = expected.toByteArray(Charsets.UTF_8)
        for (split in 0..bytes.size) {
            val decoder = StreamingUtf8Decoder()
            val actual = decoder.decode(bytes.copyOfRange(0, split)) +
                decoder.decode(byteArrayOf()) + decoder.decode(byteArrayOf()) +
                decoder.decode(bytes.copyOfRange(split, bytes.size)) + decoder.decode(byteArrayOf(), true)
            assertEquals("split=$split", expected, actual)
        }
    }
    @Test fun oneByteWritesAndEmptyPollsDoNotFinishTheDecoder() {
        val expected = "A🚀测试Z"
        val decoder = StreamingUtf8Decoder()
        val output = StringBuilder()
        expected.toByteArray(Charsets.UTF_8).forEach { byte ->
            output.append(decoder.decode(byteArrayOf(byte)))
            repeat(10) { assertEquals("", decoder.decode(byteArrayOf())) }
        }
        output.append(decoder.decode(byteArrayOf(), true))
        assertEquals(expected, output.toString())
    }
    @Test fun incompleteCodePointIsReplacedOnlyAtRealEof() {
        val decoder = StreamingUtf8Decoder()
        assertEquals("", decoder.decode(byteArrayOf(0xF0.toByte(), 0x9F.toByte())))
        repeat(20) { assertEquals("", decoder.decode(byteArrayOf())) }
        assertEquals("\uFFFD", decoder.decode(byteArrayOf(), true))
    }
    @Test fun validBytesAfterAnInvalidSequenceAreStillDecoded() {
        val decoder = StreamingUtf8Decoder()
        val output = decoder.decode(byteArrayOf(0xFF.toByte())) + decoder.decode("中文".toByteArray()) + decoder.decode(byteArrayOf(), true)
        assertEquals("\uFFFD中文", output)
    }
    @Test fun internallyBufferedPrefixSurvivesBetweenDecoderCalls() {
        val platformDecoder = PrefixConsumingDecoder()
        val decoder = StreamingUtf8Decoder(platformDecoder)
        assertEquals("", decoder.decode(byteArrayOf(0xF0.toByte(), 0x9F.toByte())))
        assertEquals(2, platformDecoder.heldBytes)
        assertEquals("", decoder.decode(byteArrayOf()))
        assertEquals("🚀", decoder.decode(byteArrayOf(0x9A.toByte(), 0x80.toByte())))
        assertEquals("", decoder.decode(byteArrayOf(), true))
    }
    @Test(expected = IllegalStateException::class) fun dataAfterEofIsRejected() {
        val decoder = StreamingUtf8Decoder()
        decoder.decode(byteArrayOf(), true)
        decoder.decode(byteArrayOf(65))
    }

    /** Models the legal ICU behavior that consumes a UTF-8 prefix into decoder-owned storage. */
    private class PrefixConsumingDecoder : CharsetDecoder(Charsets.UTF_8, 1f, 1f) {
        private val held = ArrayList<Byte>()
        val heldBytes get() = held.size
        override fun decodeLoop(input: ByteBuffer, output: CharBuffer): CoderResult {
            while (input.hasRemaining() && held.size < 4) held += input.get()
            if (held.size < 4) return CoderResult.UNDERFLOW
            if (output.remaining() < 2) return CoderResult.OVERFLOW
            output.put(held.toByteArray().toString(Charsets.UTF_8))
            held.clear()
            return CoderResult.UNDERFLOW
        }
    }
}
