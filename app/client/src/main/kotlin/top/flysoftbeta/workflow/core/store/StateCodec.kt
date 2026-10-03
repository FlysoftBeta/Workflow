package top.flysoftbeta.workflow.core.store

import top.flysoftbeta.workflow.core.json.Json
import top.flysoftbeta.workflow.core.layout.*
import top.flysoftbeta.workflow.core.resource.ComposerAttachment
import top.flysoftbeta.workflow.core.resource.ComposerDraft
import top.flysoftbeta.workflow.core.resource.DiskVersion
import top.flysoftbeta.workflow.core.resource.FileDraft
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.session.SessionId
import top.flysoftbeta.workflow.core.session.UsageStats
import java.util.Locale

class UnsupportedFormatException(message: String) : IllegalStateException(message)

/** Hand-written codecs over the strict [Json] codec. Decoders throw on malformed input. */
object StateCodec {
    const val SESSIONS_FORMAT = 1L
    const val DRAFT_FORMAT = 1L
    const val COMPOSER_FORMAT = 1L

    data class SessionsDocument(val sessions: List<Session>, val activeSessionId: SessionId?, val pinned: List<SessionId>, val dropped: Int)

    // ---- sessions.json ----

    fun encodeSessions(sessions: List<Session>, activeSessionId: SessionId?, pinned: List<SessionId>): String = Json.stringify(linkedMapOf(
        "format" to SESSIONS_FORMAT,
        "activeSessionId" to activeSessionId,
        "pinned" to pinned,
        "sessions" to sessions.map(::session),
    ))

    fun decodeSessions(text: String): SessionsDocument {
        val root = obj(Json.parse(text))
        val format = long(root, "format")
        if (format > SESSIONS_FORMAT) throw UnsupportedFormatException("sessions.json format $format is newer than this app")
        require(format == SESSIONS_FORMAT) { "Unknown sessions format $format" }
        var dropped = 0
        val sessions = list(root, "sessions").mapNotNull { item ->
            try { session(obj(item)) } catch (_: RuntimeException) { dropped++; null }
        }.distinctBy { it.id }
        val ids = sessions.map { it.id }.toSet()
        return SessionsDocument(
            sessions = sessions,
            activeSessionId = (root["activeSessionId"] as? String)?.takeIf { it in ids },
            pinned = list(root, "pinned").mapNotNull { it as? String }.filter { it in ids }.distinct(),
            dropped = dropped,
        )
    }

    private fun session(value: Session): Map<String, Any?> = linkedMapOf(
        "id" to value.id,
        "name" to value.name,
        "createdAt" to value.createdAt,
        "lastUsedAt" to value.lastUsedAt,
        "usage" to linkedMapOf("uses" to value.usage.uses, "frecency" to value.usage.frecency, "frecencyAt" to value.usage.frecencyAt),
        "archivedAt" to value.archivedAt,
        "workbench" to workbench(value.workbench),
    )

    /** A session survives a damaged layout: its workbench is reset instead. */
    private fun session(map: Map<String, Any?>): Session {
        val usage = (map["usage"] as? Map<*, *>)?.let { obj(it) }
        val createdAt = long(map, "createdAt")
        val workbench = try {
            LayoutInvariants.normalize(workbench(obj(map["workbench"])))
        } catch (_: RuntimeException) {
            Workbench.empty()
        }
        return Session(
            id = str(map, "id"),
            name = (map["name"] as? String)?.trim()?.takeIf { it.isNotEmpty() },
            createdAt = createdAt,
            lastUsedAt = (map["lastUsedAt"] as? Number)?.toLong() ?: createdAt,
            usage = if (usage == null) UsageStats.startingAt(createdAt) else UsageStats(
                uses = long(usage, "uses").toInt().coerceAtLeast(1),
                frecency = double(usage, "frecency").takeIf { it.isFinite() && it >= 0 } ?: 1.0,
                frecencyAt = long(usage, "frecencyAt"),
            ),
            archivedAt = (map["archivedAt"] as? Number)?.toLong(),
            workbench = workbench,
        )
    }

    // ---- Workbench ----

    fun workbench(w: Workbench): Map<String, Any?> = linkedMapOf(
        "paradigm" to w.paradigm.name.lowercase(Locale.ROOT),
        "panels" to w.panels.values.map { panel -> linkedMapOf("id" to panel.id, "target" to target(panel.target), "view" to view(panel.view)) },
        "stacks" to w.stacks.values.map { linkedMapOf("id" to it.id, "panels" to it.panels, "active" to it.active) },
        "editor" to node(w.editor),
        "files" to linkedMapOf(
            "explorer" to region(w.files.explorer), "aux" to region(w.files.aux), "bottom" to region(w.files.bottom),
            "focus" to w.files.focus, "maximized" to w.files.maximized,
        ),
        "chat" to linkedMapOf(
            "rail" to region(w.chat.rail), "side" to region(w.chat.side), "tree" to region(w.chat.tree),
            "sideStack" to w.chat.sideStack, "focus" to w.chat.focus,
            "promotedFrom" to w.chat.promotedFrom?.let {
                linkedMapOf("panelId" to it.panelId, "stackId" to it.stackId, "index" to it.index, "auxActiveBefore" to it.auxActiveBefore)
            },
        ),
        "solo" to w.solo?.let { linkedMapOf("panelId" to it.panelId, "returnTo" to it.returnTo.name.lowercase(Locale.ROOT)) },
        "lastEditorStack" to w.lastEditorStack,
        "explorer" to linkedMapOf("expanded" to w.explorer.expanded, "selected" to w.explorer.selected, "showHidden" to w.explorer.showHidden),
        "mru" to w.mru,
        "nextSeq" to w.nextSeq,
    )

    fun workbench(map: Map<String, Any?>): Workbench {
        val files = obj(map["files"])
        val chat = obj(map["chat"])
        val panels = linkedMapOf<PanelId, Panel>()
        list(map, "panels").forEach { item ->
            val panel = obj(item)
            val target = target(obj(panel["target"])) ?: return@forEach // unknown kinds from newer versions are dropped
            val id = str(panel, "id")
            panels[id] = Panel(id, target, view((panel["view"] as? Map<*, *>)?.let { obj(it) }))
        }
        val stacks = linkedMapOf<StackId, Stack>()
        list(map, "stacks").forEach { item ->
            val stack = obj(item)
            val id = str(stack, "id")
            stacks[id] = Stack(id, list(stack, "panels").map { it as String }, stack["active"] as? String)
        }
        val promoted = (chat["promotedFrom"] as? Map<*, *>)?.let { obj(it) }
        val solo = (map["solo"] as? Map<*, *>)?.let { obj(it) }
        val explorer = (map["explorer"] as? Map<*, *>)?.let { obj(it) }
        return Workbench(
            paradigm = enum(map, "paradigm", Paradigm.entries) ?: Paradigm.FILES,
            panels = panels,
            stacks = stacks,
            editor = node(obj(map["editor"])),
            files = FilesArrangement(
                explorer = region(files["explorer"], Region.EXPLORER), aux = region(files["aux"], Region.AUX),
                bottom = region(files["bottom"], Region.BOTTOM),
                focus = str(files, "focus"), maximized = files["maximized"] as? String,
            ),
            chat = ChatArrangement(
                rail = region(chat["rail"], Region.CHAT_RAIL), side = region(chat["side"], Region.CHAT_SIDE),
                tree = region(chat["tree"], Region.CHAT_TREE),
                sideStack = str(chat, "sideStack"), focus = str(chat, "focus"),
                promotedFrom = promoted?.let {
                    PanelOrigin(str(it, "panelId"), str(it, "stackId"), long(it, "index").toInt(), it["auxActiveBefore"] as? String)
                },
            ),
            solo = solo?.let { SoloArrangement(str(it, "panelId"), enum(it, "returnTo", Paradigm.entries) ?: Paradigm.FILES) },
            lastEditorStack = str(map, "lastEditorStack"),
            explorer = ExplorerView(
                expanded = explorer?.let { list(it, "expanded").mapNotNull { p -> p as? String } }.orEmpty(),
                selected = explorer?.get("selected") as? String,
                showHidden = explorer?.get("showHidden") == true,
            ),
            mru = list(map, "mru").mapNotNull { it as? String },
            nextSeq = (map["nextSeq"] as? Number)?.toLong() ?: 1,
        )
    }

    fun target(target: PanelTarget): Map<String, Any?> = when (target) {
        is PanelTarget.File -> linkedMapOf("kind" to "file", "path" to target.path)
        is PanelTarget.Image -> linkedMapOf("kind" to "image", "path" to target.path)
        is PanelTarget.Diff -> linkedMapOf("kind" to "diff", "path" to target.path)
        is PanelTarget.Terminal -> linkedMapOf("kind" to "terminal", "id" to target.terminalId)
        is PanelTarget.Conversation -> linkedMapOf("kind" to "conversation", "id" to target.conversationId)
        is PanelTarget.Proxy -> linkedMapOf("kind" to "proxy", "page" to target.page.name.lowercase(Locale.ROOT))
        PanelTarget.Settings -> linkedMapOf("kind" to "settings")
    }

    fun target(map: Map<String, Any?>): PanelTarget? = when (map["kind"]) {
        "file" -> PanelTarget.File(str(map, "path"))
        "image" -> PanelTarget.Image(str(map, "path"))
        "diff" -> PanelTarget.Diff(str(map, "path"))
        "terminal" -> PanelTarget.Terminal(str(map, "id"))
        "conversation" -> PanelTarget.Conversation(str(map, "id"))
        "proxy" -> PanelTarget.Proxy(enum(map, "page", ProxyPage.entries) ?: ProxyPage.OVERVIEW)
        "settings" -> PanelTarget.Settings
        else -> null
    }

    fun view(view: PanelView): Map<String, Any?> = linkedMapOf(
        "scrollAnchor" to (view.scrollAnchor ?: Json.Omit),
        "scrollOffset" to (if (view.scrollOffset == 0) Json.Omit else view.scrollOffset),
        "cursor" to (view.cursor?.let { listOf(it.line, it.column, it.anchorLine, it.anchorColumn) } ?: Json.Omit),
        "extras" to (if (view.extras.isEmpty()) Json.Omit else view.extras),
    )

    fun view(map: Map<String, Any?>?): PanelView {
        if (map == null) return PanelView()
        val cursor = (map["cursor"] as? List<*>)?.map { (it as Number).toInt() }?.takeIf { it.size == 4 }
        return PanelView(
            scrollAnchor = map["scrollAnchor"] as? String,
            scrollOffset = (map["scrollOffset"] as? Number)?.toInt() ?: 0,
            cursor = cursor?.let { TextCursor(it[0], it[1], it[2], it[3]) },
            extras = (map["extras"] as? Map<*, *>)?.entries?.mapNotNull { (k, v) -> if (k is String && v is String) k to v else null }?.toMap().orEmpty(),
        )
    }

    private fun node(node: SplitNode): Map<String, Any?> = when (node) {
        is SplitNode.Leaf -> linkedMapOf("stack" to node.stackId)
        is SplitNode.Split -> linkedMapOf(
            "split" to node.id, "axis" to node.axis.name.lowercase(Locale.ROOT),
            "children" to node.children.map(::node), "weights" to node.weights,
        )
    }

    private fun node(map: Map<String, Any?>): SplitNode = if ("stack" in map) SplitNode.Leaf(str(map, "stack")) else SplitNode.Split(
        id = str(map, "split"),
        axis = enum(map, "axis", Axis.entries) ?: Axis.ROW,
        children = list(map, "children").map { node(obj(it)) },
        weights = list(map, "weights").map { (it as Number).toDouble() },
    )

    private fun region(state: RegionState): Map<String, Any?> = linkedMapOf("size" to state.size, "collapsed" to state.collapsed)

    private fun region(value: Any?, region: Region): RegionState {
        val map = (value as? Map<*, *>)?.let { obj(it) } ?: return RegionState(region.defaultSize)
        return RegionState((map["size"] as? Number)?.toDouble() ?: region.defaultSize, map["collapsed"] as? Boolean ?: false)
    }

    // ---- Drafts and composers ----

    fun encodeDraft(draft: FileDraft): String = Json.stringify(linkedMapOf(
        "format" to DRAFT_FORMAT,
        "path" to draft.path,
        "text" to draft.text,
        "base" to version(draft.base),
        "editedAt" to draft.editedAt,
        "revision" to draft.revision,
    ))

    fun decodeDraft(text: String): FileDraft {
        val map = obj(Json.parse(text))
        val format = long(map, "format")
        if (format > DRAFT_FORMAT) throw UnsupportedFormatException("draft format $format")
        return FileDraft(str(map, "path"), str(map, "text"), version(obj(map["base"])), long(map, "editedAt"), long(map, "revision").coerceAtLeast(1))
    }

    fun encodeComposer(draft: ComposerDraft): String = Json.stringify(linkedMapOf(
        "format" to COMPOSER_FORMAT,
        "conversationId" to draft.conversationId,
        "revision" to draft.revision,
        "text" to draft.text,
        "attachments" to draft.attachments.map { linkedMapOf("path" to it.path, "mimeType" to (it.mimeType ?: Json.Omit)) },
    ))

    fun decodeComposer(text: String): ComposerDraft {
        val map = obj(Json.parse(text))
        val format = long(map, "format")
        if (format > COMPOSER_FORMAT) throw UnsupportedFormatException("composer format $format")
        return ComposerDraft(
            conversationId = str(map, "conversationId"),
            revision = long(map, "revision"),
            text = str(map, "text"),
            attachments = list(map, "attachments").map { item -> obj(item).let { ComposerAttachment(str(it, "path"), it["mimeType"] as? String) } },
        )
    }

    fun version(version: DiskVersion): Map<String, Any?> = linkedMapOf(
        "exists" to version.exists, "size" to version.size, "modifiedAt" to version.modifiedAt, "sha256" to version.sha256,
    )

    fun version(map: Map<String, Any?>): DiskVersion = DiskVersion(
        exists = map["exists"] as? Boolean ?: error("version.exists"),
        size = (map["size"] as? Number)?.toLong() ?: -1,
        modifiedAt = (map["modifiedAt"] as? Number)?.toLong() ?: -1,
        sha256 = map["sha256"] as? String,
    )

    // ---- Small helpers ----

    @Suppress("UNCHECKED_CAST")
    fun obj(value: Any?): Map<String, Any?> = value as? Map<String, Any?> ?: error("Expected a JSON object")
    fun str(map: Map<String, Any?>, key: String): String = map[key] as? String ?: error("Expected string $key")
    fun long(map: Map<String, Any?>, key: String): Long = (map[key] as? Number)?.toLong() ?: error("Expected number $key")
    fun double(map: Map<String, Any?>, key: String): Double = (map[key] as? Number)?.toDouble() ?: error("Expected number $key")
    fun list(map: Map<String, Any?>, key: String): List<Any?> = map[key] as? List<*> ?: emptyList()
    fun <E : Enum<E>> enum(map: Map<String, Any?>, key: String, values: List<E>): E? =
        (map[key] as? String)?.let { value -> values.firstOrNull { it.name.equals(value, ignoreCase = true) } }
}
