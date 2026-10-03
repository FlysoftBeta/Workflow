package top.flysoftbeta.workflow.core.io

import java.security.MessageDigest

object Hashing {
    fun sha256(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256").digest(bytes).toHex()
    fun sha256(text: String): String = sha256(text.toByteArray(Charsets.UTF_8))

    private val digits = "0123456789abcdef".toCharArray()
    private fun ByteArray.toHex(): String {
        val out = CharArray(size * 2)
        forEachIndexed { index, byte ->
            val value = byte.toInt() and 0xff
            out[index * 2] = digits[value ushr 4]
            out[index * 2 + 1] = digits[value and 0x0f]
        }
        return String(out)
    }
}
