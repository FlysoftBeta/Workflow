package top.flysoftbeta.workflow.core.connection

import java.io.IOException
import top.flysoftbeta.workflow.core.config.ConfigCodec
import top.flysoftbeta.workflow.core.config.ConfigParse
import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.resource.*
import top.flysoftbeta.workflow.core.store.*
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.session.UsageStats

/** Current wire format only. Protocol negotiation rejects other revisions before these decoders run. */
object WorkspaceWire {
    fun obj(value: Any?): Map<String, Any?> = WorkspaceRpc.obj(value)
    fun string(map: Map<String, Any?>, key: String) = map[key] as? String ?: throw IOException("Missing $key")
    fun long(map: Map<String, Any?>, key: String): Long = when (val value = map[key]) {
        is Long -> value
        is Int -> value.toLong()
        else -> throw IOException("Missing or invalid integer $key")
    }
    fun encode(draft: ComposerDraft): Map<String, Any?> = obj(Json.parse(StateCodec.encodeComposer(draft)))
    fun composer(value: Any?): ComposerDraft = StateCodec.decodeComposer(Json.stringify(value))
    fun version(value: Any?): DiskVersion = StateCodec.version(obj(value))
    fun draft(value: Any?): FileDraft? = value?.let { StateCodec.decodeDraft(Json.stringify(it)) }

    fun state(value: Any?): WorkspaceState {
        val map = obj(value)
        val sessions = list(map, "sessions").map { value ->
            val session = obj(value)
            val usage = obj(session["usage"])
            val uses = long(usage, "uses")
            val frecency = (usage["frecency"] as? Number)?.toDouble() ?: throw IOException("Missing frecency")
            check(uses in 1..Int.MAX_VALUE.toLong() && frecency.isFinite() && frecency >= 0) { "Invalid session usage" }
            Session(string(session, "id"), nullableString(session, "name"), long(session, "createdAt"),
                long(session, "lastUsedAt"), UsageStats(uses.toInt(), frecency, long(usage, "frecencyAt")),
                session["archivedAt"]?.let { long(session, "archivedAt") }, workbench(session["workbench"]))
        }
        val ids = sessions.map { it.id }.toSet()
        val active = nullableString(map, "activeSessionId")
        val pinned = list(map, "pinned").map { it as? String ?: throw IOException("Invalid pinned session") }
        check(ids.size == sessions.size && (active == null || active in ids) && pinned.distinct() == pinned && pinned.all { it in ids }) {
            "Workspace returned invalid session references"
        }
        val config = ConfigCodec.decode(Json.stringify(map["config"])) as? ConfigParse.Ok
            ?: throw IOException("Workspace returned invalid effective configuration")
        val status = when (map["status"]) {
            "ready" -> StoreStatus.READY
            "failed" -> StoreStatus.FAILED
            else -> throw IOException("Workspace returned unknown status")
        }
        fun values(key: String) = obj(map[key])
        return WorkspaceState(
            status = status, failure = map["failure"] as? String,
            sessions = sessions, activeSessionId = active, pinned = pinned,
            drafts = values("drafts").mapValues { draft(it.value) ?: throw IOException("Empty draft") },
            composers = values("composers").mapValues { composer(it.value) },
            disk = values("disk").mapValues { version(it.value) },
            config = config.config, configProblem = map["configProblem"] as? String,
            notices = (map["notices"] as? List<*>).orEmpty().map { item ->
                val notice = obj(item)
                StoreNotice(string(notice, "id"), NoticeKind.valueOf(string(notice, "kind").uppercase()), string(notice, "message"))
            },
            writeError = map["writeError"] as? String,
        )
    }

    private fun list(map: Map<String, Any?>, key: String): List<*> = map[key] as? List<*> ?: throw IOException("Missing $key")
    private fun nullableString(map: Map<String, Any?>, key: String): String? = map[key]?.let { it as? String ?: throw IOException("Invalid $key") }

    /** Decode an authoritative layout without running the local repair/normalization reducer. */
    fun workbench(value: Any?): Workbench {
        val map = obj(value)
        check(map["paradigm"] in setOf("files", "chat", "solo")) { "Invalid workspace paradigm" }
        val panels = list(map, "panels")
        panels.forEach { value ->
            val target = obj(obj(value)["target"])
            check(StateCodec.target(target) != null) { "Unknown workspace panel target" }
            if (target["kind"] == "proxy") check(target["page"] in setOf("overview", "logs", "connections")) { "Unknown proxy page" }
        }
        fun node(value: Any?) {
            val node = obj(value)
            if ("stack" !in node) {
                check(node["axis"] in setOf("row", "column")) { "Unknown workspace split axis" }
                list(node, "children").forEach(::node)
            }
        }
        node(map["editor"])
        val decoded = StateCodec.workbench(map)
        check(decoded.panels.size == panels.size && decoded.stacks.size == list(map, "stacks").size) { "Duplicate workspace layout ids" }
        check(LayoutInvariants.violations(decoded).isEmpty()) { "Workspace returned invalid layout" }
        return decoded
    }

    fun file(value: Any?): FileSnapshot {
        val map = obj(value)
        return FileSnapshot(string(map, "path"), version(map["disk"]), map["diskText"] as? String,
            map["binary"] == true, map["tooLarge"] == true, draft(map["draft"]))
    }

    fun resource(value: Any?): ResourceRef = obj(value).let {
        when (it["kind"]) {
            "file" -> ResourceRef.File(string(it, "path"))
            "conversation" -> ResourceRef.Conversation(string(it, "id"))
            else -> throw IOException("Unknown working resource")
        }
    }

    private fun tag(type: String, vararg fields: Pair<String, Any?>): Map<String, Any?> = linkedMapOf("type" to type, *fields)
    private fun Enum<*>.wire() = name.lowercase()

    fun layout(op: LayoutOp): Map<String, Any?> = when (op) {
        is LayoutOp.Open -> tag("open", "target" to StateCodec.target(op.target), "placement" to placement(op.placement), "focus" to op.focus)
        is LayoutOp.Focus -> tag("focus", "panelId" to op.panelId)
        is LayoutOp.FocusStack -> tag("focusStack", "stackId" to op.stackId)
        is LayoutOp.Close -> tag("close", "panelIds" to op.panelIds)
        is LayoutOp.Move -> tag("move", "panelId" to op.panelId, "to" to drop(op.to))
        is LayoutOp.SplitStack -> tag("splitStack", "stackId" to op.stackId, "edge" to op.edge.wire())
        is LayoutOp.ResizeSplit -> tag("resizeSplit", "splitId" to op.splitId, "weights" to op.weights)
        is LayoutOp.ResetSplit -> tag("resetSplit", "splitId" to op.splitId)
        is LayoutOp.ResizeRegion -> tag("resizeRegion", "region" to op.region.wire(), "size" to op.size)
        is LayoutOp.SetRegionCollapsed -> tag("setRegionCollapsed", "region" to op.region.wire(), "collapsed" to op.collapsed)
        is LayoutOp.ToggleRegion -> tag("toggleRegion", "region" to op.region.wire())
        is LayoutOp.SetMaximized -> tag("setMaximized", "stackId" to op.stackId)
        is LayoutOp.SwitchParadigm -> tag("switchParadigm", "paradigm" to op.paradigm.wire())
        is LayoutOp.PromoteConversation -> tag("promoteConversation", "panelId" to op.panelId)
        LayoutOp.ReturnToFiles -> tag("returnToFiles")
        is LayoutOp.EnterSolo -> tag("enterSolo", "target" to StateCodec.target(op.target))
        is LayoutOp.SetChatSideStack -> tag("setChatSideStack", "stackId" to op.stackId)
        is LayoutOp.Retarget -> tag("retarget", "panelId" to op.panelId, "target" to StateCodec.target(op.target))
        is LayoutOp.UpdateView -> tag("updateView", "panelId" to op.panelId, "view" to StateCodec.view(op.view))
        is LayoutOp.UpdateExplorer -> tag("updateExplorer", "explorer" to mapOf("expanded" to op.explorer.expanded, "selected" to op.explorer.selected, "showHidden" to op.explorer.showHidden))
        is LayoutOp.RenamePath -> tag("renamePath", "from" to op.from, "to" to op.to)
    }

    private fun placement(value: Placement): Map<String, Any?> = when (value) {
        Placement.Auto -> tag("auto")
        is Placement.InStack -> tag("inStack", "stackId" to value.stackId, "index" to value.index, "replaceActive" to value.replaceActive, "moveExisting" to value.moveExisting)
        is Placement.SplitEdge -> tag("splitEdge", "stackId" to value.stackId, "edge" to value.edge.wire())
    }

    private fun drop(value: DropTarget): Map<String, Any?> = when (value) {
        is DropTarget.Center -> tag("center", "stackId" to value.stackId)
        is DropTarget.Tab -> tag("tab", "stackId" to value.stackId, "index" to value.index)
        is DropTarget.Edge -> tag("edge", "stackId" to value.stackId, "edge" to value.edge.wire())
        is DropTarget.EditorEdge -> tag("editorEdge", "edge" to value.edge.wire())
    }
}
