package top.flysoftbeta.workflow.proxy.runtime

import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.RandomAccessFile
import java.security.MessageDigest
import java.util.UUID
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.long
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.proxy.config.LocalProxyConfig
import top.flysoftbeta.workflow.proxy.guardian.ProxyProcessIdentity
import top.flysoftbeta.workflow.proxy.io.readProxyBytes

/**
 * Disposable executor staging directory (Android cache; never the canonical workspace):
 * - `config.yaml`: an Engine-delivered copy of the user configuration;
 * - `.process.json`: ephemeral identity of the last started kernel, for read-only diagnosis only;
 * - `runtime.log` / `runtime.previous.log`: secret-redacted kernel output, rotated at 1 MiB;
 * - everything else (cache.db, providers, geo databases) belongs to Mihomo, which runs with `-d` here.
 */
class ProxyFiles(directory: File) {
    val directory: File = directory.absoluteFile
    val config = File(this.directory, "config.yaml")
    val record = File(this.directory, ".process.json")
    val log = File(this.directory, "runtime.log")
    val previousLog = File(this.directory, "runtime.previous.log")
    private val recordLock = Any()
    private val logLock = Any()
    private val configLock = Any()

    fun ensureDirectory() { check(directory.mkdirs() || directory.isDirectory) { "无法创建代理目录" } }

    fun readConfig(): ByteArray {
        check(config.isFile) { "还没有代理配置" }
        check(config.length() <= LocalProxyConfig.MAX_CONFIG_BYTES) { "代理配置超过 16 MiB" }
        return config.inputStream().use { readProxyBytes(it, LocalProxyConfig.MAX_CONFIG_BYTES) }
    }

    /** Explicit user import: atomic replacement of the whole file. */
    fun replaceConfig(bytes: ByteArray) = synchronized(configLock) {
        require(bytes.isNotEmpty()) { "代理配置不能为空" }
        require(bytes.size <= LocalProxyConfig.MAX_CONFIG_BYTES) { "代理配置超过 16 MiB" }
        atomicWrite(config, bytes)
    }

    fun persistIdentity(identity: ProxyProcessIdentity) = synchronized(recordLock) {
        val data = buildJsonObject {
            put("pid", identity.pid); put("startTime", identity.startTime)
            put("guardPid", identity.guardPid); put("guardStartTime", identity.guardStartTime)
            put("runId", identity.runId); put("executable", identity.executable)
            put("directory", identity.directory); put("config", identity.config)
        }
        atomicWrite(record, data.toString().toByteArray(Charsets.UTF_8))
    }

    /** Null if absent or not a record for this directory. Records are never used as authority to signal a process. */
    fun readIdentity(): ProxyProcessIdentity? = synchronized(recordLock) {
        runCatching {
            if (!record.isFile || record.length() >= 16384) return@runCatching null
            val data = Json.parseToJsonElement(record.inputStream().use { readProxyBytes(it, 16384) }.toString(Charsets.UTF_8)) as JsonObject
            fun text(key: String) = (data[key] as JsonPrimitive).also { require(it.isString) }.content
            require(text("directory") == directory.path && text("config") == config.path)
            val executable = text("executable")
            require(executable.startsWith("/") && executable.endsWith("/libmihomo.so"))
            ProxyProcessIdentity((data["pid"] as JsonPrimitive).int, (data["startTime"] as JsonPrimitive).long,
                (data["guardPid"] as JsonPrimitive).int, (data["guardStartTime"] as JsonPrimitive).long,
                text("runId"), executable, text("directory"), text("config"))
        }.getOrNull()
    }

    fun hasRecord(): Boolean = record.exists()

    fun removeIdentity(runId: String) = synchronized(recordLock) {
        if (readIdentity()?.runId == runId) check(record.delete() || !record.exists()) { "无法清理已停止代理的进程记录" }
    }

    fun removeRecord() = synchronized(recordLock) { check(record.delete() || !record.exists()) { "无法清理代理进程记录" } }

    fun appendLog(text: String) = synchronized(logLock) {
        if (text.isEmpty()) return@synchronized
        runCatching {
            ensureDirectory()
            if (log.length() > MAX_LOG_BYTES) { previousLog.delete(); log.renameTo(previousLog) }
            FileOutputStream(log, true).use { it.write(text.toByteArray(Charsets.UTF_8)) }
        }
        Unit
    }

    /** The last [maxBytes] of runtime.log (already redacted when written). */
    fun logTail(maxBytes: Int = 64 * 1024): String = synchronized(logLock) {
        require(maxBytes in 1..(1024 * 1024)) { "日志读取范围须为 1–1048576 字节" }
        if (!log.isFile) return ""
        RandomAccessFile(log, "r").use { file ->
            val start = (file.length() - maxBytes).coerceAtLeast(0)
            file.seek(start)
            val bytes = ByteArray((file.length() - start).toInt())
            file.readFully(bytes)
            String(bytes, Charsets.UTF_8).let { if (start > 0) it.substringAfter('\n') else it }
        }
    }

    private fun atomicWrite(file: File, bytes: ByteArray) {
        ensureDirectory()
        val temporary = File(file.parentFile, ".${file.name}.${UUID.randomUUID()}.tmp")
        try {
            FileOutputStream(temporary).use { it.write(bytes); it.fd.sync() }
            if (!temporary.renameTo(file)) throw IOException("无法保存 ${file.name}")
        } finally { temporary.delete() }
    }

    companion object {
        const val MAX_LOG_BYTES = 1024 * 1024L
        fun digest(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it.toInt() and 255) }
    }
}
