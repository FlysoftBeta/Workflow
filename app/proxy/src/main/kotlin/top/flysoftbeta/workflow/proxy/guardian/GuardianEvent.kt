package top.flysoftbeta.workflow.proxy.guardian

import java.io.IOException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.longOrNull

/** One JSONL record from `libworkflow_proxy_guard.so` stdout (see app/native/proxy-guard/README.md). */
sealed interface GuardianEvent {
    data class Started(val fields: Map<String, Any?>) : GuardianEvent
    data class Error(val message: String, val fatal: Boolean) : GuardianEvent
    data class Exit(val fields: Map<String, Any?>) : GuardianEvent

    companion object {
        const val MAX_LINE = 4096

        /** Strict: unknown events, oversized lines and malformed JSON are protocol errors. */
        fun parse(line: String): GuardianEvent {
            if (line.length > MAX_LINE) throw IOException("守护进程控制记录过长")
            val entry = try { Json.parseToJsonElement(line) as? JsonObject } catch (_: Exception) { null }
                ?: throw IOException("守护进程控制记录无效")
            fun value(key: String): Any? {
                val primitive = entry[key] as? JsonPrimitive ?: return null
                return when {
                    primitive.isString -> primitive.content
                    primitive.booleanOrNull != null -> primitive.booleanOrNull
                    else -> primitive.longOrNull
                }
            }
            return when (value("event")) {
                "started" -> Started(listOf("uid", "pid", "startTime", "guardPid", "guardStartTime", "runId").associateWith(::value))
                "error" -> Error((value("message") as? String)?.take(300) ?: "代理守护进程错误", value("fatal") as? Boolean ?: true)
                "exit" -> Exit(listOf("pid", "exitCode", "forced").associateWith(::value))
                else -> throw IOException("未知守护进程控制记录")
            }
        }
    }
}
