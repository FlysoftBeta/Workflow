package top.flysoftbeta.workflow.proxy.io

import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.InputStream

/** API 28 has no InputStream.readNBytes; enforce the limit even if length changes while reading. */
fun readProxyBytes(input: InputStream, maximum: Int): ByteArray {
    require(maximum >= 0)
    val output = ByteArrayOutputStream(minOf(maximum, 8192))
    val buffer = ByteArray(8192)
    while (true) {
        val count = input.read(buffer, 0, minOf(buffer.size.toLong(), maximum.toLong() - output.size() + 1).toInt())
        if (count < 0) return output.toByteArray()
        if (count == 0) {
            val next = input.read()
            if (next < 0) return output.toByteArray()
            if (output.size() == maximum) throw IOException("读取内容超过大小限制")
            output.write(next)
        } else {
            if (count > maximum - output.size()) throw IOException("读取内容超过大小限制")
            output.write(buffer, 0, count)
        }
    }
}
