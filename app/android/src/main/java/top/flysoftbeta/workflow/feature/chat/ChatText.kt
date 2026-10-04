package top.flysoftbeta.workflow.feature.chat

import java.time.DayOfWeek
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.temporal.ChronoUnit
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.Decision
import top.flysoftbeta.workflow.agent.model.DecisionKind
import top.flysoftbeta.workflow.agent.model.ItemStatus
import top.flysoftbeta.workflow.agent.model.MarkerKind
import top.flysoftbeta.workflow.agent.model.Notice
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.RequestKind
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.TurnError

/** User-visible chat wording in one place (Chinese UI; no protocol names). Pure, unit-tested. */
object ChatText {
    fun backendName(kind: BackendKind) = when (kind) {
        BackendKind.CODEX -> "Codex"
        BackendKind.CLAUDE -> "Claude Code"
    }

    fun permissionLabel(preset: PermissionPreset) = when (preset) {
        PermissionPreset.ASK -> "需确认"
        PermissionPreset.AUTO_EDIT -> "自动编辑"
        PermissionPreset.PLAN -> "仅规划"
        PermissionPreset.DENY_UNLISTED -> "仅限已允许"
    }

    fun permissionDescription(preset: PermissionPreset) = when (preset) {
        PermissionPreset.ASK -> "运行命令或修改文件前询问"
        PermissionPreset.AUTO_EDIT -> "可直接修改工作区文件，命令仍需确认"
        PermissionPreset.PLAN -> "只讨论与规划，不执行"
        PermissionPreset.DENY_UNLISTED -> "不询问，未允许的操作一律拒绝"
    }

    private val EFFORTS = linkedMapOf(
        "none" to "无", "minimal" to "极低", "low" to "低", "medium" to "中", "high" to "高", "xhigh" to "极高",
        "max" to "最高", "ultra" to "Ultra",
    )

    fun effortLabel(id: String?): String = id?.let { EFFORTS[it] ?: it } ?: "默认"

    fun notice(notice: Notice): String = when (notice.code) {
        "approvalsReviewerNotUser" -> "此对话的审批没有交给你，已停止发送"
        "usageLimitExceeded" -> "已达到使用上限，请稍后再试"
        else -> notice.message
    }

    fun turnError(error: TurnError?): String = when (error?.code) {
        "usageLimitExceeded" -> "已达到使用上限，请稍后再试"
        "sendFailed" -> "发送失败"
        else -> error?.message?.lineSequence()?.firstOrNull()?.take(200) ?: "出错了"
    }

    private fun verb(status: ItemStatus, running: String, done: String, declined: String = "未$done") = when (status) {
        ItemStatus.IN_PROGRESS -> running
        ItemStatus.DECLINED -> declined
        else -> done
    }

    fun runTitle(command: String, status: ItemStatus) = verb(status, "正在运行", "已运行", "未运行") + " " + command.lineSequence().first().take(160)
    fun readTitle(names: List<String>, status: ItemStatus) = verb(status, "正在读取", "已读取") + " " + names.joinToString("、").take(160)
    fun listTitle(path: String, status: ItemStatus) = verb(status, "正在列出", "已列出") + " " + path.ifEmpty { "工作区" }
    fun searchTitle(query: String, status: ItemStatus) = verb(status, "正在搜索", "已搜索") + " " + query.take(120)
    fun filesTitle(names: List<String>, status: ItemStatus) = verb(status, "正在编辑", "已编辑", "未修改") + " " + names.joinToString("、").take(160)
    fun toolTitle(server: String?, tool: String, status: ItemStatus) = verb(status, "正在调用", "已调用") + " " + listOfNotNull(server, tool).joinToString(".")

    /** `/bin/zsh -lc "git status"` → `git status`. */
    fun unwrapShell(command: String): String {
        val match = SHELL_WRAPPER.matchEntire(command.trim()) ?: return command.trim()
        val inner = match.groupValues[2]
        return when (match.groupValues[1]) {
            "\"" -> inner.replace("\\\"", "\"").replace("\\\\", "\\")
            else -> inner.replace("'\\''", "'")
        }
    }

    private val SHELL_WRAPPER = Regex("""^(?:\S*/)?(?:ba|z|da)?sh\s+-l?c\s+(["'])([\s\S]*)\1$""")

    fun marker(kind: MarkerKind, text: String?) = when (kind) {
        MarkerKind.COMPACTION -> "已压缩上下文"
        MarkerKind.REVIEW_ENTERED -> "进入审阅"
        MarkerKind.REVIEW_EXITED -> "退出审阅"
        MarkerKind.HOOK, MarkerKind.HOOK_PROMPT -> "钩子" + (text?.let { "：$it" } ?: "")
        MarkerKind.MODEL_REROUTED -> "已切换模型" + (text?.let { "：$it" } ?: "")
        MarkerKind.INTERRUPTED -> "已中断"
        MarkerKind.LOCAL_COMMAND -> text ?: "本地命令"
        MarkerKind.SLEEP -> "等待" + (text?.let { " $it" } ?: "")
    }

    /** Card title (docs/ux/README.md §4.7). */
    fun requestTitle(kind: RequestKind): String = when (kind) {
        is RequestKind.CommandApproval -> if (kind.networkHost != null) "访问网络" else "运行命令"
        is RequestKind.FileChangeApproval -> "修改文件"
        is RequestKind.PermissionsApproval -> "权限"
        is RequestKind.ToolApproval -> when (kind.tool) {
            "Bash" -> "运行命令"
            "Edit", "Write", "MultiEdit", "NotebookEdit" -> "修改文件"
            else -> "工具请求"
        }
        is RequestKind.UserInput -> "需要你回答"
        is RequestKind.PlanApproval -> "批准计划"
        is RequestKind.Elicitation -> "需要你回答"
        is RequestKind.UserDialog -> "未识别的请求"
        is RequestKind.Unknown -> "未识别的请求"
    }

    /** One-line subject of a request ("运行 git status"). */
    fun requestSubject(kind: RequestKind): String = when (kind) {
        is RequestKind.CommandApproval -> kind.networkHost?.let { "访问 $it" } ?: ("运行 " + unwrapShell(kind.command ?: "命令").lineSequence().first().take(120))
        is RequestKind.FileChangeApproval -> if (kind.changes.isEmpty()) "修改文件" else "修改 " + kind.changes.joinToString("、") { it.path.substringAfterLast('/') }.take(120)
        is RequestKind.PermissionsApproval -> "额外权限"
        is RequestKind.ToolApproval -> kind.title ?: kind.displayName ?: kind.tool
        is RequestKind.UserInput -> kind.questions.firstOrNull()?.question?.take(80) ?: "问题"
        is RequestKind.PlanApproval -> "计划"
        is RequestKind.Elicitation -> kind.message.lineSequence().firstOrNull()?.take(80) ?: "表单"
        is RequestKind.UserDialog -> "未识别的请求"
        is RequestKind.Unknown -> "未识别的请求"
    }

    fun decisionLabel(decision: Decision, kind: RequestKind): String = when (decision.kind) {
        DecisionKind.ALLOW_ONCE -> if (kind is RequestKind.PlanApproval) "批准计划" else "批准"
        DecisionKind.ALLOW_SESSION -> "本对话内允许"
        DecisionKind.ALLOW_PERSISTENT -> "始终允许"
        DecisionKind.DENY -> "拒绝"
        DecisionKind.ABORT -> "拒绝并停止本轮"
        DecisionKind.OTHER -> readableId(decision.id)
    }

    private fun readableId(id: String): String = id.replace(Regex("([a-z])([A-Z])"), "$1 $2").replace('_', ' ').lowercase()

    /** Transcript line for a settled request ("已批准 · 运行 git status"). */
    fun requestRecord(request: PendingRequest): String {
        val subject = requestSubject(request.kind)
        val verb = when (request.status) {
            RequestStatus.EXPIRED -> "已失效"
            RequestStatus.CANCELLED -> "已撤回"
            RequestStatus.REJECTED -> "已拒绝"
            RequestStatus.RESOLVED -> "已处理"
            RequestStatus.PENDING -> "待确认"
            RequestStatus.ANSWERED -> {
                val decision = request.decisions.firstOrNull { it.id == request.answer }
                when {
                    decision != null -> when (decision.kind) {
                        DecisionKind.ALLOW_ONCE -> "已批准"
                        DecisionKind.ALLOW_SESSION -> "已在本对话内允许"
                        DecisionKind.ALLOW_PERSISTENT -> "已始终允许"
                        DecisionKind.DENY -> "已拒绝"
                        DecisionKind.ABORT -> "已拒绝并停止"
                        DecisionKind.OTHER -> "已选择"
                    }
                    request.answer == "accept" -> "已提交"
                    request.answer == "decline" -> "已拒绝"
                    request.answer == "cancel" -> "已取消"
                    request.answer == "rejected" -> "已拒绝"
                    else -> "已回答"
                }
            }
        }
        return "$verb · $subject"
    }

    fun timeSeparator(time: LocalDateTime, today: LocalDate): String {
        val date = time.toLocalDate()
        val clock = "%d:%02d".format(time.hour, time.minute)
        val days = ChronoUnit.DAYS.between(date, today)
        return when {
            days == 0L -> clock
            days == 1L -> "昨天 $clock"
            days in 2..6 -> weekday(date.dayOfWeek) + " " + clock
            date.year == today.year -> "${date.monthValue}月${date.dayOfMonth}日 $clock"
            else -> "${date.year}年${date.monthValue}月${date.dayOfMonth}日 $clock"
        }
    }

    fun weekday(day: DayOfWeek) = when (day) {
        DayOfWeek.MONDAY -> "星期一"
        DayOfWeek.TUESDAY -> "星期二"
        DayOfWeek.WEDNESDAY -> "星期三"
        DayOfWeek.THURSDAY -> "星期四"
        DayOfWeek.FRIDAY -> "星期五"
        DayOfWeek.SATURDAY -> "星期六"
        DayOfWeek.SUNDAY -> "星期日"
    }

    /** Compact age for the conversation list ("刚刚", "5 分钟", "3 小时", "昨天", "3 天", "9月2日"). */
    fun relative(epochMs: Long, nowMs: Long, today: LocalDate, date: LocalDate): String {
        val minutes = (nowMs - epochMs) / 60_000
        val days = ChronoUnit.DAYS.between(date, today)
        return when {
            minutes < 1 -> "刚刚"
            minutes < 60 -> "$minutes 分钟"
            days == 0L -> "${minutes / 60} 小时"
            days == 1L -> "昨天"
            days < 7 -> "$days 天"
            date.year == today.year -> "${date.monthValue}月${date.dayOfMonth}日"
            else -> "${date.year}/${date.monthValue}/${date.dayOfMonth}"
        }
    }

    /** Rail group of a date (docs/ux/README.md §3.4). */
    fun railGroup(date: LocalDate, today: LocalDate): String {
        val days = ChronoUnit.DAYS.between(date, today)
        return when {
            days <= 0L -> "今天"
            days == 1L -> "昨天"
            days < 7 -> "近 7 天"
            else -> "更早"
        }
    }

    /** Title of an untitled conversation: its first message's first line. */
    fun untitled(preview: String?): String = preview?.lineSequence()?.firstOrNull { it.isNotBlank() }?.trim()?.take(60) ?: "新对话"
}
