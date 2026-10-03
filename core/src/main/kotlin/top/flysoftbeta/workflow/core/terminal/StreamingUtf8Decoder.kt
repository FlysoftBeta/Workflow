package top.flysoftbeta.workflow.core.terminal

import java.nio.ByteBuffer
import java.nio.CharBuffer
import java.nio.charset.CharsetDecoder
import java.nio.charset.CodingErrorAction

/**
 * One decoder per stream. Android's ICU decoder can consume an incomplete prefix into internal
 * state; retaining only ByteBuffer.remaining() while creating a new decoder loses those bytes.
 * Other JVM decoders leave the prefix in the input buffer, so preserve that remainder as well.
 */
class StreamingUtf8Decoder(
    private val decoder: CharsetDecoder = Charsets.UTF_8.newDecoder(),
) {
    private var remainder = byteArrayOf()
    private var ended = false
    init { decoder.onMalformedInput(CodingErrorAction.REPLACE).onUnmappableCharacter(CodingErrorAction.REPLACE) }

    fun decode(bytes: ByteArray, endOfInput: Boolean = false): String {
        check(!ended) { "UTF-8 stream has already ended" }
        if (bytes.isEmpty() && !endOfInput) return ""
        val input = ByteBuffer.wrap(remainder + bytes)
        val output = CharBuffer.allocate((input.remaining() + 4).coerceAtLeast(8))
        val text = StringBuilder()
        fun drain() { output.flip(); text.append(output); output.clear() }
        while (true) {
            val result = decoder.decode(input, output, endOfInput)
            drain()
            if (result.isError) result.throwException()
            if (result.isUnderflow) break
        }
        remainder = ByteArray(input.remaining()).also { input.get(it) }
        if (endOfInput) {
            while (true) {
                val result = decoder.flush(output)
                drain()
                if (result.isError) result.throwException()
                if (result.isUnderflow) break
            }
            ended = true
            check(remainder.isEmpty()) { "UTF-8 decoder left bytes after EOF" }
        }
        return text.toString()
    }
}
