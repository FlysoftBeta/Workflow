package top.flysoftbeta.workflow.core.connection

import java.io.IOException

/** A capability the operation needs is blocked until the user acts, such as a failed environment. */
open class CapabilityBlockedException(message: String) : IOException(message)

/**
 * How a failed operation is presented (docs/ux/design-system.md, "Readiness and failures"). [summary] is short
 * user-facing text; [detail] keeps the technical message for Details only.
 */
sealed interface Failure {
    val summary: String
    val detail: String?

    /** Resolves without the user, such as a slow Engine. The surface keeps showing what it waits for. */
    data class Transient(override val summary: String, override val detail: String?) : Failure

    /** The connection is gone. Its status card already explains that, so the action ends without a report. */
    data class Lost(override val detail: String?) : Failure {
        override val summary get() = "工作区连接已断开"
    }

    /** A capability is blocked until the user acts where that capability is shown. */
    data class Blocked(override val summary: String, override val detail: String?) : Failure

    /** Engine refused this request for a stated reason. */
    data class Rejected(val kind: String?, override val summary: String, override val detail: String?) : Failure

    /** Anything else: an unexpected local failure or a defect. */
    data class Unexpected(override val detail: String?) : Failure {
        override val summary get() = "操作没有完成"
    }

    companion object {
        fun of(error: Throwable): Failure = when (error) {
            is WorkspaceClosedException -> Lost(error.message)
            is WorkspaceTimeoutException -> Transient(error.message ?: "工作区响应超时", "${error.method} (${error.timeoutMs} ms)")
            is CapabilityBlockedException -> Blocked(error.message ?: "功能暂不可用", null)
            is WorkspaceRpcException -> when (error.kind) {
                "environment_preparing" -> Transient("正在准备环境", error.message)
                "closed" -> Lost(error.message)
                else -> Rejected(error.kind, rejection(error.kind), error.message)
            }
            else -> Unexpected(error.message ?: error.javaClass.simpleName)
        }

        /** Short text for Engine's stable error kinds; Details keeps Engine's own message. */
        fun rejection(kind: String?): String = when (kind) {
            "conflict", "stale_operation" -> "内容已在别处更改"
            "exists" -> "同名项目已存在"
            "not_found" -> "找不到该项目"
            "read_only" -> "该位置只读"
            "too_large", "limit", "overflow", "name_limit" -> "超出大小限制"
            "invalid_params", "invalid_text", "invalid_image" -> "请求内容无效"
            "unavailable" -> "该功能当前不可用"
            "io" -> "读写失败"
            "spawn_failed" -> "无法启动进程"
            else -> "操作没有完成"
        }
    }
}
