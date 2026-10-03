package top.flysoftbeta.workflow.engine.chat

import java.io.File
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.*
import top.flysoftbeta.workflow.agent.SendMode
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.rpc.*
import top.flysoftbeta.workflow.core.connection.*
import top.flysoftbeta.workflow.core.resource.ComposerAttachment
import top.flysoftbeta.workflow.core.store.StateCodec
import top.flysoftbeta.workflow.core.store.StoreStatus

/** All conversation policy and adapter lifetime lives here, behind the Rust Workspace endpoint. */
class ChatService(private val rpc: WorkspaceRpc, private val scope: CoroutineScope) {
    private val store = RemoteWorkspaceStore(rpc, scope, "engine-chat")
    private val launcher = GuestProcessLauncher()
    private val hub = AgentHub(store, scope, Dispatchers.IO)
    private val journal = ChatJournal()
    private val ledger = SendLedger(rpc, store)
    @Volatile private var closed = false
    private val installed = mutableSetOf<BackendKind>()
    private val toolsGate = Mutex()
    private val demanded = mutableSetOf<BackendKind>()
    private var claudeInstallRequested = false

    suspend fun start() {
        store.start()
        check(store.awaitReady().status == StoreStatus.READY) { "Workspace not ready for chat" }
        hub.events.addListener(journal::event)
        hub.start()
        hub.index.awaitLoaded()
        refreshTools()
        publishMetadata()
        scope.launch {
            combine(hub.conversations, hub.available, store.state) { _, _, _ -> Unit }.collect { publishMetadata() }
        }
        scope.launch {
            store.state.map { it.config.agent.backend }.distinctUntilChanged().collect {
                try { refreshTools() } catch (cancelled: CancellationException) { throw cancelled }
                catch (_: Exception) { /* A selected tool's measured failure remains visible until explicit retry. */ }
            }
        }
        scope.launch {
            while (isActive) {
                delay(3000)
                try { refreshTools() } catch (cancelled: CancellationException) { throw cancelled }
                catch (_: Exception) { /* A later measured status may recover. The public transport owns failure. */ }
            }
        }
    }
    private fun publishMetadata() = journal.metadata(ChatMetadata(
        conversations = hub.conversations.value,
        available = hub.available.value,
        loginMethods = BackendKind.entries.associateWith(hub::loginMethods),
        permissions = hub.permissions(),
        defaultBackend = hub.defaultBackend(),
    ))
    private suspend fun refreshTools(required: BackendKind? = null) = toolsGate.withLock {
        if (closed) return@withLock
        required?.let(demanded::add)
        var status = WorkspaceRpc.obj(rpc.request("environment.tools.status"))
        if (closed) return@withLock
        fun tools() = (status["tools"] as? List<*>)?.map { WorkspaceRpc.obj(it) }.orEmpty()
        val claude = tools().firstOrNull { it["id"] == BackendKind.CLAUDE.id }
        val needsClaude = hub.defaultBackend() == BackendKind.CLAUDE || BackendKind.CLAUDE in demanded
        if (needsClaude && claude?.get("phase") == "not_installed" && !claudeInstallRequested) {
            // Mark before dispatch: an ambiguous callback failure must not create an install storm.
            // Failed installs remain failed; only the explicit tools retry action may retry them.
            claudeInstallRequested = true
            status = WorkspaceRpc.obj(rpc.request("environment.tools.install", mapOf("toolId" to "claude", "retry" to false)))
        }
        val tools = tools()
        for (kind in BackendKind.entries) {
            if (closed) return@withLock
            val tool = tools.firstOrNull { it["id"] == kind.id }
            val ready = tool?.get("phase") == "ready"
            if (ready && kind !in installed) {
                val binary = "/opt/workflow/tools/${kind.id}/bin/${kind.id}"
                check(tool.get("binary") == binary) { "Unexpected managed agent executable" }
                check(withContext(Dispatchers.IO) { File(binary).canExecute() }) { "Managed agent is not executable" }
                val home = "/home/work/.${kind.id}"
                withContext(Dispatchers.IO) {
                    java.nio.file.Files.createDirectories(File(home).toPath())
                    java.nio.file.Files.setPosixFilePermissions(File(home).toPath(), java.nio.file.attribute.PosixFilePermissions.fromString("rwx------"))
                }
                hub.replaceBackend(kind, BackendSetup(launcher, binary, "/workspace", home, "/tmp", guestEnvironment(), ::readGuestFile))
                installed += kind
            } else if (!ready && kind in installed) {
                hub.replaceBackend(kind, null)
                installed -= kind
            }
        }
    }
    private fun guestEnvironment(): Map<String, String> {
        // Explicit variables only. Never inherit API credentials or dynamic-loader injection.
        val denied = listOf("OPENAI_", "CODEX_", "ANTHROPIC_", "CLAUDE_", "CLAUDECODE", "LD_PRELOAD", "LD_LIBRARY_PATH")
        val result = System.getenv().filterKeys { name -> denied.none(name::startsWith) }.toMutableMap()
        result.putAll(mapOf("HOME" to "/home/work", "USER" to "work", "LOGNAME" to "work", "SHELL" to "/bin/bash", "TERM" to "dumb"))
        result.putIfAbsent("LANG", "C.UTF-8")
        result.putIfAbsent("PATH", "/opt/workflow/bin:/usr/local/bin:/usr/bin:/bin")
        return result
    }
    private suspend fun readGuestFile(path: String, maxBytes: Int): ByteArray? = withContext(Dispatchers.IO) {
        require(maxBytes in 0..20 * 1024 * 1024)
        val file = File(path).canonicalFile
        val p = file.path
        require(p.startsWith("/home/work/") || (p.startsWith("/workspace/") && p != "/workspace/.workspace" && !p.startsWith("/workspace/.workspace/"))) { "Agent resource is outside permitted guest roots" }
        if (!file.isFile) return@withContext null
        file.inputStream().use { input ->
            val bytes = input.readNBytes(maxBytes + 1)
            require(bytes.size <= maxBytes) { "Agent resource exceeds read limit" }
            bytes
        }
    }
    suspend fun request(method: String, args: JsonObject): JsonElement = when (method) {
        "chat.snapshot" -> journal.snapshotResult(args.stringOrNull("transferId"), args["offset"]?.jsonPrimitive?.int ?: 0)
        "chat.watch" -> ChatWire.encode(journal.watch(args.string("epoch"), args["afterRevision"]!!.jsonPrimitive.long, args["timeoutMs"]?.jsonPrimitive?.long ?: 25_000))
        "chat.command" -> command(args.string("name"), args["args"]?.jsonObject ?: JsonObject(emptyMap()))
        else -> throw IllegalArgumentException("Unknown chat method")
    }
    private suspend fun command(name: String, a: JsonObject): JsonElement {
        fun id() = a.string("id")
        fun kind(field: String = "kind") = backend(a.string(field))
        var result: JsonElement = JsonNull
        when (name) {
            "newConversation" -> {
                val created = hub.newConversation(a.stringOrNull("backend")?.let(::backend), a.stringOrNull("id") ?: java.util.UUID.randomUUID().toString())
                refreshTools(hub.entry(created)?.backend)
                result = JsonPrimitive(created)
            }
            "ensureConversation" -> {
                val entry = hub.ensureConversation(id())
                refreshTools(entry.backend)
                result = ChatWire.encode(entry)
            }
            "open" -> {
                refreshTools(hub.ensureConversation(id()).backend)
                hub.open(id())
            }
            "loadEarlier" -> hub.loadEarlier(id())
            "setBackend" -> { hub.setBackend(id(), kind()); refreshTools(kind()) }
            "send" -> {
                val submitted = StateCodec.decodeComposer(a.getValue("submitted").toString())
                val attachments = a["attachments"]?.jsonArray.orEmpty().map { it.jsonObject.let { v -> ComposerAttachment(v.string("path"), v.stringOrNull("mimeType")) } }
                val text = a.string("text")
                require(submitted.conversationId == id() && submitted.text == text && submitted.attachments == attachments) { "Submitted composer does not match the message" }
                val settings = a["settings"]?.let { ChatWire.decode<TurnSettings>(it) } ?: TurnSettings()
                val mode = a.stringOrNull("mode")?.let { SendMode.valueOf(it.uppercase()) } ?: SendMode.AUTO
                result = JsonPrimitive(ledger.send(a.string("operationId"), a, submitted) {
                    hub.send(id(), text, attachments, settings, mode)
                })
                hub.rememberSelection(id(), settings.model, settings.effort)
            }
            "interrupt" -> hub.interrupt(id())
            "cancelQueued" -> hub.cancelQueued(id(), a.string("clientMessageId"))
            "respond" -> {
                val key = ChatWire.decode<RequestKey>(a.getValue("key"))
                journal.validateResponse(key, a.string("processEpoch"))
                hub.respond(key, ChatWire.decode(a.getValue("response")))
            }
            "rename" -> hub.rename(id(), a.string("title"))
            "archive" -> hub.archive(id(), a.getValue("archived").jsonPrimitive.boolean)
            "deleteConversation" -> hub.deleteConversation(id())
            "compact" -> hub.compact(id())
            "refreshUsage" -> hub.refreshUsage(kind())
            "refreshModels" -> result = ChatWire.encode(hub.connect(kind()).refreshModels())
            "rawRequest" -> result = hub.rawRequest(id(), a.string("method"), a["params"]?.takeUnless { it is JsonNull })
            "fork" -> result = JsonPrimitive(hub.fork(id(), a.stringOrNull("atTurnId")))
            "setPermissions" -> hub.setPermissions(id(), PermissionPreset.valueOf(a.string("preset").uppercase()))
            "rememberSelection" -> hub.rememberSelection(id(), a.stringOrNull("model"), a.stringOrNull("effort"))
            "login" -> result = ChatWire.encode<LoginFlow>(hub.login(kind(), LoginMethod.valueOf(a.string("method").uppercase()), a.stringOrNull("secret")))
            "cancelLogin" -> hub.cancelLogin(kind(), a.string("loginId"))
            "logout" -> hub.logout(kind())
            "refreshAccount" -> hub.refreshAccount(kind())
            "warmUp" -> { refreshTools(kind()); hub.connect(kind()) }
            "flush" -> hub.flush()
            else -> throw IllegalArgumentException("Unknown chat command: $name")
        }
        publishMetadata()
        return result
    }
    suspend fun close() {
        closed = true
        try { withTimeoutOrNull(5000) { hub.close() } } finally { launcher.close() }
    }
}

internal fun JsonObject.string(name: String): String = stringOrNull(name) ?: throw IllegalArgumentException("Missing $name")
internal fun JsonObject.stringOrNull(name: String): String? = this[name]?.takeUnless { it is JsonNull }?.jsonPrimitive?.content
internal fun backend(value: String): BackendKind = BackendKind.of(value.lowercase()) ?: throw IllegalArgumentException("Unknown backend")
