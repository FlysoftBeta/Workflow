package top.flysoftbeta.workflow.platform.pty

import androidx.annotation.Keep

/** Byte arrays carry standard UTF-8 instead of JNI's modified UTF-8 (including non-BMP paths). */
@Keep
internal object NativePty {
    init { System.loadLibrary("workflow_pty") }

    @JvmStatic external fun spawn(cwd: ByteArray, arguments: Array<ByteArray>, environment: Array<ByteArray>, rows: Int, columns: Int): LongArray
    /** null means EOF, an empty array means the bounded poll timed out. */
    @JvmStatic external fun read(handle: Long, timeoutMs: Int): ByteArray?
    @JvmStatic external fun write(handle: Long, bytes: ByteArray): Int
    @JvmStatic external fun resize(handle: Long, rows: Int, columns: Int)
    @JvmStatic external fun waitFor(handle: Long): Int
    @JvmStatic external fun stop(handle: Long, graceMs: Int)
    @JvmStatic external fun release(handle: Long)
}
