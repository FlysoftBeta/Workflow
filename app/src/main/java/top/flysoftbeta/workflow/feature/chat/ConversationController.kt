package top.flysoftbeta.workflow.feature.chat

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.conflate
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import top.flysoftbeta.workflow.agent.SendMode
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.BackendStatus
import top.flysoftbeta.workflow.agent.model.ConversationEntry
import top.flysoftbeta.workflow.agent.model.LoginMethod
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.MessagePhase
import top.flysoftbeta.workflow.agent.model.PendingRequest
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.ProcessState
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.SliderPosition
import top.flysoftbeta.workflow.agent.model.ThreadState
import top.flysoftbeta.workflow.agent.model.Turn
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UserMessageItem
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.app.panel.DropAffordance
import top.flysoftbeta.workflow.app.panel.PanelContext
import top.flysoftbeta.workflow.app.panel.PanelController
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.app.panel.PanelTab
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.TextCursor
import top.flysoftbeta.workflow.feature.chat.transcript.TranscriptCallbacks
import top.flysoftbeta.workflow.feature.chat.transcript.NativeTranscriptState
import top.flysoftbeta.workflow.feature.chat.transcript.NativeTranscriptDocument
import top.flysoftbeta.workflow.feature.chat.transcript.NativeTranscriptProjector
import top.flysoftbeta.workflow.feature.chat.transcript.TranscriptProjector
import top.flysoftbeta.workflow.platform.agent.AgentHub
import top.flysoftbeta.workflow.platform.agent.BackendUnavailableException
import top.flysoftbeta.workflow.platform.importer.ImportKind
import top.flysoftbeta.workflow.platform.importer.ImportResult
import top.flysoftbeta.workflow.platform.importer.ImportService
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.ToolAction
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DragPayload
import top.flysoftbeta.workflow.ui.design.dnd.ExternalDragPayload
import top.flysoftbeta.workflow.ui.design.dnd.FilesDragPayload
import top.flysoftbeta.workflow.ui.design.icons.Sym

/**
 * One conversation panel (docs/ui.md §4.5–§4.8): the native transcript, pinned request cards and
 * the composer. The shell retains this controller and its viewport across panel reattachment.
 */
class ConversationController(
    val conversationId: String,
    internal val context: PanelContext,
    internal val services: ChatFeatureServices,
) : PanelController {
    internal val hub: AgentHub get() = services.hub
    internal val importer: ImportService get() = services.importer
    internal val composer = ComposerModel(conversationId, context.store, services, context.scope)

    var entry by mutableStateOf<ConversationEntry?>(null)
        private set
    var thread by mutableStateOf<ThreadState?>(null)
        private set
    var backend by mutableStateOf<BackendStatus?>(null)
        private set
    /** Requests waiting for this conversation's user, in arrival order. */
    var openRequests by mutableStateOf<List<PendingRequest>>(emptyList())
        private set
    var available by mutableStateOf(false)
        private set
    var atBottom by mutableStateOf(true)
        private set
    /** One-line problem shown above the composer (send failed, backend failed …). */
    var problem by mutableStateOf<String?>(null)
    var selection by mutableStateOf<SliderPosition?>(null)
        private set
    var permissions by mutableStateOf(hub.permissions())
        private set
    var showAllConversations by mutableStateOf(false)
    var renaming by mutableStateOf(false)
    var toolsPage by mutableStateOf<String?>(null)
    var confirmingDelete by mutableStateOf(false)

    private var savedView = context.initialView
    private val expanded = mutableStateMapOf<String, Boolean>().apply {
        context.initialView.extras.filterKeys { it.startsWith("chat.expanded:") }.forEach { (key, value) ->
            put(key.removePrefix("chat.expanded:"), value == "true")
        }
    }
    private val expandedFlow = MutableStateFlow<Map<String, Boolean>>(expanded.toMap())
    internal val transcriptState = NativeTranscriptState(context.initialView)
    internal var transcript by mutableStateOf(NativeTranscriptDocument())
        private set
    private val resetRequests = MutableStateFlow(0)

    init {
        context.scope.launch {
            val e = hub.ensureConversation(conversationId)
            entry = e
            // Start failures surface as backend state (未能启动 / 需要工作环境), not as an action error.
            runCatching { hub.open(conversationId) }
        }
        context.scope.launch {
            hub.conversations.map { list -> list.firstOrNull { it.id == conversationId } }.distinctUntilChanged().collect { e ->
                if (e != null) {
                    val previousBackend = entry?.backend
                    entry = e
                    if (previousBackend != null && previousBackend != e.backend) resetRequests.value++
                }
            }
        }
        context.scope.launch {
            combine(hub.state, hub.available, hub.conversations) { state, avail, _ -> state to avail }.conflate().collect { (state, avail) ->
                val e = entry ?: return@collect
                available = e.backend in avail
                backend = state.backend(e.backend)
                val key = hub.threadKey(e)
                thread = key?.let { state.thread(it) }
                openRequests = state.requests.values.filter { r ->
                    r.status.isOpen && r.key.backend == e.backend && (r.threadId == null || r.threadId == key?.id)
                }
                selection = resolveSelection(e, state)
                // Login finished or the process restarted: make sure the thread is live again.
                if (key != null && state.backend(e.backend).process is ProcessState.Ready &&
                    state.backend(e.backend).account.state == LoginState.LOGGED_IN && state.thread(key)?.turns.isNullOrEmpty()) {
                    context.scope.launch { runCatching { hub.open(conversationId) } }
                }
            }
        }
        context.scope.launch { transcriptLoop() }
        // A pick that finished after the process was recreated still lands in this conversation.
        context.scope.launch {
            importer.orphaned.collect { (request, result) ->
                if (request.owner == importOwner && result is ImportResult.Imported) composer.addPaths(result.paths)
            }
        }
    }

    private val importOwner get() = "conversation:$conversationId"

    // ------------------------------------------------------------------ panel contract

    override val tab: PanelTab
        get() = PanelTab(title, backendIcon(entry?.backend), dirty = composer.hasContent)

    val title: String get() = entry?.let { it.title ?: ChatText.untitled(it.preview) } ?: "新对话"

    override val actions: List<ToolAction>
        get() = listOf(ToolAction("newConversation", Sym.EditSquare, "新对话") { context.commands.newConversation() })

    override val resourceMenu: List<MenuEntry>
        get() {
            val e = entry
            val hasThread = e?.backendThreadId != null
            return listOfNotNull(
                MenuEntry.Action("rename", "重命名", Sym.Edit) { renaming = true },
                MenuEntry.Action("fork", "分叉", Sym.ForkRight, enabled = hasThread && thread?.activeTurn == null) { fork(null) },
                MenuEntry.Action("copyMarkdown", "复制为 Markdown", Sym.Markdown, enabled = !thread?.turns.isNullOrEmpty()) { copyTranscript() },
                MenuEntry.Action("compact", "压缩上下文", Sym.UnfoldLess, enabled = hasThread && available && thread?.activeTurn == null) {
                    launchCatching { hub.compact(conversationId) }
                },
                MenuEntry.Action("usage", "用量与后端状态", Sym.Info) { toolsPage = "usage" },
                MenuEntry.Action("console", "高级控制台", Sym.Terminal, enabled = available) { toolsPage = "console" },
                if (e?.archived == true) MenuEntry.Action("unarchive", "取消归档", Sym.Unarchive) { archive(false) }
                else MenuEntry.Action("archive", "归档", Sym.Archive, enabled = e != null) { archive(true) },
                MenuEntry.Action("delete", "删除对话…", Sym.Delete, enabled = e != null && thread?.activeTurn == null, destructive = true) { confirmingDelete = true },
            )
        }

    override val switcher: List<MenuGroup>
        get() {
            val recent = hub.conversations.value.filter { !it.archived && it.id != conversationId }
                .sortedByDescending { it.updatedAtMs }.take(8)
            return listOf(
                MenuGroup("new", listOf(MenuEntry.Action("new", "新对话", Sym.EditSquare) { context.commands.newConversation() })),
                MenuGroup("recent", recent.map { e ->
                    MenuEntry.Action("c:${e.id}", e.title ?: ChatText.untitled(e.preview), backendIcon(e.backend)) { switchTo(e.id) }
                }),
                MenuGroup("all", listOf(MenuEntry.Action("all", "全部对话…", Sym.History) { showAllConversations = true })),
            )
        }

    override val needsAttention: Boolean get() = openRequests.isNotEmpty()

    @Composable
    override fun Content(frame: PanelFrame, modifier: Modifier) {
        ConversationContent(this, frame, modifier)
    }

    override fun dropAffordance(payload: DragPayload): DropAffordance? = when (payload) {
        is FilesDragPayload, is ExternalDragPayload -> DropAffordance(AreaStyle.Outline, "添加为附件")
        else -> null
    }

    override fun onDrop(payload: DragPayload) {
        when (payload) {
            is FilesDragPayload -> composer.addPaths(payload.paths.filter { it.isNotEmpty() })
            is ExternalDragPayload -> importUris(payload.uris, payload.label)
            else -> Unit
        }
    }

    override fun requestInputFocus() {
        focusRequests.value++
    }

    internal val focusRequests = MutableStateFlow(0)

    override fun dispose() {
        services.processScope.launch { runCatching { composer.save() } }
    }

    // ------------------------------------------------------------------ transcript

    private suspend fun transcriptLoop() {
        var projector: TranscriptProjector? = null
        var projectorBackend: BackendKind? = null
        val native = NativeTranscriptProjector()
        combine(hub.state, expandedFlow, resetRequests, hub.conversations) { state, exp, _, _ -> state to exp }
            .conflate()
            .collect { (state, exp) ->
                val e = entry ?: return@collect
                if (projector == null || projectorBackend != e.backend) {
                    projector = TranscriptProjector(hub.paths(e.backend))
                    projectorBackend = e.backend
                }
                val key = hub.threadKey(e)
                val t = key?.let { state.thread(it) }
                val requests = if (key == null) emptyList() else state.requestsFor(key)
                transcript = withContext(Dispatchers.Default) {
                    native.project(projector!!.project(t, requests, exp), t?.historyCursor != null)
                }
                // Conflate protocol bursts before the next parse/layout, with no per-token main-thread work.
                delay(64)
            }
    }

    internal val callbacks = object : TranscriptCallbacks {
        override fun openUrl(url: String) {
            if (top.flysoftbeta.workflow.feature.chat.transcript.NativeMarkdownParser.safeLink(url) == null) return
            runCatching {
                context.appContext.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            }.onFailure { context.commands.snackbar("没有可以打开此链接的应用") }
        }

        override fun openPath(path: String, line: Int?, column: Int?) {
            val paths = entry?.let { hub.paths(it.backend) } ?: return
            val link = paths.parseLink(path, line, column)
            if (link == null) {
                context.commands.snackbar("无法打开此路径")
                return
            }
            context.commands.openFile(link.path, link.line?.let { TextCursor(it, link.column ?: 1) })
        }

        override fun copyText(text: String) = copy(text)

        override fun copyTurn(turn: String, what: String) {
            val t = findTurn(turn) ?: return
            val text = when (what) {
                "user" -> t.items.filterIsInstance<UserMessageItem>().firstOrNull()?.text
                "markdown" -> TranscriptMarkdown.turn(t)
                else -> t.finalMessage?.text?.toString() ?: t.items.filterIsInstance<AgentMessageItem>().lastOrNull()?.text?.toString()
            }
            if (!text.isNullOrEmpty()) copy(text)
        }

        override fun toggle(turn: String, expanded: Boolean) {
            this@ConversationController.expanded[turn] = expanded
            expandedFlow.value = this@ConversationController.expanded.toMap()
            savedView = savedView.copy(extras = savedView.extras + ("chat.expanded:$turn" to expanded.toString()))
            context.updateView(savedView)
        }

        override fun earlier() = launchCatching { hub.loadEarlier(conversationId) }

        override fun layout(atBottom: Boolean) {
            this@ConversationController.atBottom = atBottom
        }

        override suspend fun resource(path: String, maxBytes: Int): ByteArray? = services.readResource(path, maxBytes)

        override fun viewport(anchor: String?, offset: Int, atBottom: Boolean) {
            savedView = savedView.copy(scrollAnchor = anchor, scrollOffset = offset,
                extras = savedView.extras + ("chat.follow" to atBottom.toString()))
            context.updateView(savedView)
        }

        override fun action(turn: String, name: String) {
            val t = findTurn(turn) ?: return
            when (name) {
                "fork" -> if (t.status.isFinal && t.bound) fork(t.id)
                "retry" -> if (t.status == TurnStatus.FAILED) retry(t)
            }
        }
    }

    private fun findTurn(viewId: String): Turn? = thread?.turns?.firstOrNull { (it.clientMessageId ?: it.id) == viewId }

    fun scrollToBottom() {
        transcriptState.bottom()
        atBottom = true
    }

    // ------------------------------------------------------------------ actions

    val running: Boolean get() = thread?.turns?.any { it.status == TurnStatus.RUNNING } == true

    /** Queued follow-ups shown as chips (the next one is already shown in the transcript while idle). */
    val queued: List<Turn>
        get() {
            val turns = thread?.turns ?: return emptyList()
            val q = turns.filter { it.status == TurnStatus.QUEUED }
            return if (running) q else q.drop(1)
        }

    /** Sending is blocked: the backend reported approvals are not routed to the user. */
    val reviewerBlocked: Boolean
        get() = thread?.settings?.approvalsReviewer?.let { it != "user" } == true

    fun send(mode: SendMode = SendMode.AUTO) {
        val e = entry ?: return
        if (!composer.hasContent || composer.importing) return
        problem = null
        context.scope.launch {
            val stored = composer.save()
            val (text, attachments) = composer.takeForSend()
            val settings = TurnSettings(model = selection?.model, effort = selection?.effort, permissions = permissions)
            try {
                hub.send(e.id, text, attachments, settings, mode)
                composer.acknowledge(stored)
                selection?.let { hub.rememberSelection(e.id, it.model, it.effort) }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                composer.restore(text, attachments.map { it.path })
                report(error)
            }
        }
    }

    fun stop() = launchCatching { hub.interrupt(conversationId) }

    fun cancelQueued(turn: Turn) = launchCatching { turn.clientMessageId?.let { hub.cancelQueued(conversationId, it) } }

    fun respond(request: PendingRequest, response: RequestResponse) = launchCatching { hub.respond(request.key, response) }

    fun select(position: SliderPosition) {
        selection = position
        launchCatching { hub.rememberSelection(conversationId, position.model, position.effort) }
    }

    fun setPermissionPreset(preset: PermissionPreset) {
        permissions = preset
        launchCatching { hub.setPermissions(conversationId, preset) }
    }

    fun setBackend(kind: BackendKind) = launchCatching { hub.setBackend(conversationId, kind) }

    fun login(method: LoginMethod, secret: String? = null) = launchCatching { hub.login(entry?.backend ?: return@launchCatching, method, secret) }

    fun cancelLogin(loginId: String) = launchCatching { hub.cancelLogin(entry?.backend ?: return@launchCatching, loginId) }

    fun retryBackend() {
        problem = null
        context.scope.launch { runCatching { hub.open(conversationId) } }
    }

    fun rename(title: String) = launchCatching { hub.rename(conversationId, title) }

    fun archive(archived: Boolean) = launchCatching {
        hub.archive(conversationId, archived)
        if (archived) context.commands.snackbar("已归档", "撤销") { launchCatching { hub.archive(conversationId, false) } }
    }

    fun fork(turnId: String?) = launchCatching {
        val created = hub.fork(conversationId, turnId)
        switchTo(created)
    }

    fun switchTo(id: String) {
        context.layout(LayoutOp.Retarget(context.panelId, PanelTarget.Conversation(id)))
    }

    private fun retry(turn: Turn) {
        val user = turn.items.filterIsInstance<UserMessageItem>().firstOrNull() ?: return
        val paths = entry?.let { hub.paths(it.backend) }
        val files = user.parts.mapNotNull { part ->
            when (part) {
                is UserPart.Image -> paths?.toWorkspace(part.path)
                is UserPart.File -> paths?.toWorkspace(part.path)
                else -> null
            }
        }
        composer.restore(user.text, files)
        send()
    }

    /** Drops from other apps: imported into the workspace, then attached. */
    internal fun importUris(uris: List<Uri>, label: String) {
        if (uris.isEmpty()) return
        val key = composer.beginImport(label)
        context.scope.launch { finishImport(key, importer.importUris(uris, attachmentDirectory())) }
    }

    /** "+" → 拍照 / 相册 / 设备文件 through the shared importer (same code path as the explorer's upload). */
    internal fun pick(kind: ImportKind) {
        context.scope.launch {
            val result = importer.pick(kind, attachmentDirectory(), importOwner)
            if (result is ImportResult.Cancelled) return@launch
            finishImport(composer.beginImport(if (kind == ImportKind.CAMERA) "照片" else "文件"), result)
        }
    }

    private fun finishImport(key: String, result: ImportResult) {
        when (result) {
            is ImportResult.Imported -> {
                composer.finishImport(key, result.paths)
                if (result.failures.isNotEmpty()) context.commands.snackbar("无法导入 ${result.failures.joinToString("、")}")
            }
            is ImportResult.Failed -> { composer.failImport(key); problem = result.message }
            ImportResult.Cancelled -> composer.finishImport(key, emptyList())
        }
    }

    private fun copyTranscript() {
        val t = thread ?: return
        copy(TranscriptMarkdown.thread(t, title))
    }

    internal fun copy(text: String) {
        val clipboard = context.appContext.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("Workflow", text))
        context.commands.snackbar("已复制")
    }

    private fun resolveSelection(e: ConversationEntry, state: AgentState): SliderPosition? {
        val catalog = state.backend(e.backend).models ?: return selection
        val current = selection ?: SliderPosition(e.model ?: "", e.effort)
        return catalog.resolve(current)
    }

    internal fun launchCatching(block: suspend () -> Unit) {
        context.scope.launch {
            try {
                block()
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                report(error)
            }
        }
    }

    /** Technical text of the last failure, for [详情] only. */
    var problemDetail by mutableStateOf<String?>(null)

    private fun report(error: Throwable) {
        val process = backend?.process
        // Start failures are shown by the backend state itself ("… 未能启动" / "需要工作环境").
        if (error is BackendUnavailableException || process is ProcessState.Failed || process is ProcessState.Exited || !available) {
            problem = null
            return
        }
        problemDetail = error.message
        problem = when (error) {
            is IllegalArgumentException -> "无法发送：内容无效"
            else -> "操作没有完成"
        }
    }

    companion object {
        fun backendIcon(kind: BackendKind?): Int = when (kind) {
            BackendKind.CLAUDE -> Sym.Asterisk
            BackendKind.CODEX -> Sym.Hexagon
            null -> Sym.Chat
        }
    }
}

/** Plain Markdown export ("复制为 Markdown"). */
object TranscriptMarkdown {
    fun turn(turn: Turn): String = buildString {
        turn.items.filterIsInstance<UserMessageItem>().firstOrNull()?.let { append("**用户：**\n\n").append(it.text.trim()).append("\n\n") }
        val answer = turn.finalMessage ?: turn.items.filterIsInstance<AgentMessageItem>().lastOrNull { it.phase != MessagePhase.COMMENTARY }
        answer?.let { append(it.text.toString().trim()).append("\n") }
    }.trim()

    fun thread(thread: ThreadState, title: String): String = buildString {
        append("# ").append(title).append("\n\n")
        thread.turns.filter { it.status.isFinal || it.status == TurnStatus.RUNNING }.forEach { append(turn(it)).append("\n\n---\n\n") }
    }.trimEnd().removeSuffix("---").trimEnd() + "\n"
}
