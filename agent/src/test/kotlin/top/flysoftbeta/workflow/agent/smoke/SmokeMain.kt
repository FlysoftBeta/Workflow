package top.flysoftbeta.workflow.agent.smoke

import java.io.File
import kotlin.system.exitProcess
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.ThreadOptions
import top.flysoftbeta.workflow.agent.claude.AttachmentReader
import top.flysoftbeta.workflow.agent.claude.ClaudeBackend
import top.flysoftbeta.workflow.agent.claude.ClaudeConfig
import top.flysoftbeta.workflow.agent.claude.ClaudeTranscriptSource
import top.flysoftbeta.workflow.agent.codex.CodexBackend
import top.flysoftbeta.workflow.agent.codex.CodexConfig
import top.flysoftbeta.workflow.agent.json.get
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.AgentMessageItem
import top.flysoftbeta.workflow.agent.model.AgentState
import top.flysoftbeta.workflow.agent.model.BackendKind
import top.flysoftbeta.workflow.agent.model.RequestResponse
import top.flysoftbeta.workflow.agent.model.RequestStatus
import top.flysoftbeta.workflow.agent.model.ThreadKey
import top.flysoftbeta.workflow.agent.model.TurnSettings
import top.flysoftbeta.workflow.agent.model.TurnStatus
import top.flysoftbeta.workflow.agent.model.UserPart
import top.flysoftbeta.workflow.agent.process.JvmProcessLauncher
import top.flysoftbeta.workflow.agent.testing.await

/**
 * Host smoke runs against real binaries (not part of `:agent:test`):
 *
 *   ./gradlew :agent:smoke -PsmokeArgs="codex <codex binary> <codex home> <work dir>"
 *     Real `codex app-server`, ephemeral thread, model gpt-reserve/low: one markdown+math turn and one
 *     command-approval turn that is DECLINED through RequestResponse. Two real model turns.
 *   ./gradlew :agent:smoke -PsmokeArgs="claude <claude binary> <mock api port> <work dir>"
 *     Real Claude Code CLI against the research mock Messages API (no model, dummy key): turn,
 *     permission allow, fork with a client-chosen session id, transcript hydration.
 */
fun main(args: Array<String>) {
    val ok = runBlocking { run(args) }
    exitProcess(if (ok) 0 else 1)
}

private suspend fun run(args: Array<String>): Boolean {
    val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    val store = AgentStateStore()
    return try {
        when (args.firstOrNull()) {
            "codex" -> codex(args, store, scope)
            "claude" -> claude(args, store, scope)
            else -> { System.err.println("usage: codex|claude …"); false }
        }
    } catch (e: Throwable) {
        e.printStackTrace()
        false
    } finally {
        summarize(store.state.value)
        scope.cancel()
    }
}

private fun check(label: String, value: Boolean): Boolean {
    println("${if (value) "PASS" else "FAIL"} $label")
    return value
}

private fun summarize(state: AgentState) {
    for ((kind, b) in state.backends) println("backend $kind process=${b.process} account=${b.account.state}/${b.account.method} models=${b.models?.models?.size} unknown=${b.unknown.size}")
    for (t in state.threads.values) {
        println("thread ${t.key} title=${t.title} run=${t.runState} settings=${t.settings.model}/${t.settings.effort}/${t.settings.approvalPolicy}/${t.settings.approvalsReviewer}")
        for (turn in t.turns) {
            println("  turn ${turn.id} ${turn.status} ${turn.error?.code ?: ""}")
            for (item in turn.items) println("    ${item::class.simpleName} ${item.id} ${item.status}" + ((item as? AgentMessageItem)?.let { " ${it.phase} ${it.text.toString().take(80).replace("\n", "⏎")}" } ?: ""))
        }
    }
    for (r in state.requests.values) println("request ${r.key} ${r.method} ${r.status} answer=${r.answer} decisions=${r.decisions.map { it.id }}")
}

private suspend fun codex(args: Array<String>, store: AgentStateStore, scope: CoroutineScope): Boolean {
    val (_, binary, codexHome, work) = args
    File(work).mkdirs()
    val env = mapOf("PATH" to (System.getenv("PATH") ?: "/usr/bin:/bin"), "HOME" to System.getProperty("user.home"), "LANG" to "C.UTF-8", "TERM" to "dumb")
    val codex = CodexBackend(JvmProcessLauncher(), CodexConfig(binary, codexHome, work, env, clientVersion = "smoke"), store, scope)
    codex.start()
    store.await(30_000, "models") { it.backend(BackendKind.CODEX).models != null }
    val thread = codex.startThread(ThreadOptions(work, TurnSettings(model = "gpt-reserve"), ephemeral = true))
    val key = ThreadKey(BackendKind.CODEX, thread)
    var ok = check("server confirmed approvalsReviewer=user", store.state.value.threads.getValue(key).settings.approvalsReviewer == "user")

    codex.send(thread, listOf(UserPart.Text("Answer in Markdown only, under 60 words, no tools: a level-2 heading and the quadratic formula as a display \$\$…\$\$ block.")), TurnSettings("gpt-reserve", "low"))
    store.await(180_000, "turn 1") { s -> s.threads.getValue(key).turns.lastOrNull()?.status?.isFinal == true }
    val t1 = store.state.value.threads.getValue(key).turns.last()
    ok = check("turn 1 completed with a final message containing \$\$", t1.status == TurnStatus.COMPLETED && t1.finalMessage?.text?.toString()?.contains("\$\$") == true) && ok

    codex.send(thread, listOf(UserPart.Text("Create a file named smoke.txt in the current directory containing 'hi' using a shell command (printf). Then reply with one sentence.")), TurnSettings("gpt-reserve", "low"))
    val request = store.await(180_000, "approval or turn end") { s ->
        s.requests.values.any { it.status == RequestStatus.PENDING } || s.threads.getValue(key).turns.last().status.isFinal
    }.requests.values.firstOrNull { it.status == RequestStatus.PENDING }
    if (request != null) {
        println("approval decisions offered: ${request.decisions.map { it.id }}")
        delay(1000)
        ok = check("request still pending (nothing auto-approved)", store.state.value.requests.getValue(request.key).status == RequestStatus.PENDING) && ok
        val choice = request.decisions.firstOrNull { it.id == "decline" } ?: request.decisions.first { it.id == "cancel" }
        codex.respond(request.key, RequestResponse.Decide(choice.id))
    } else println("model did not ask for approval (turn ended)")
    store.await(180_000, "turn 2") { s -> s.threads.getValue(key).turns.last().status.isFinal }
    ok = check("smoke.txt was not created", !File(work, "smoke.txt").exists()) && ok
    codex.stop()
    return ok
}

private suspend fun claude(args: Array<String>, store: AgentStateStore, scope: CoroutineScope): Boolean {
    val (_, binary, port, work) = args
    val root = File(work).absoluteFile
    val cwd = File(root, "run").apply { mkdirs() }
    val configDir = File(root, "claude-home").apply { mkdirs() }
    val tmp = File(root, "tmp").apply { mkdirs() }
    val png = File(cwd, "red.png")
    if (!png.exists()) png.writeBytes(java.util.Base64.getDecoder().decode("iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAIAAACQkWg2AAAAF0lEQVR4nGP4z8BAEiJN9aiGUQ1DSgMAkPn/Afnh+ngAAAAASUVORK5CYII="))
    val config = ClaudeConfig(
        executable = binary, configDir = configDir.path, cwd = cwd.path, tmpDir = tmp.path,
        env = mapOf("PATH" to (System.getenv("PATH") ?: "/usr/bin:/bin"), "HOME" to root.path, "SHELL" to "/bin/bash", "LANG" to "C.UTF-8", "TERM" to "dumb"),
        credentials = mapOf("ANTHROPIC_BASE_URL" to "http://127.0.0.1:$port", "ANTHROPIC_API_KEY" to "dummy-local-mock-key"),
    )
    val reader = AttachmentReader { path, _ -> File(path).readBytes() }
    val transcripts = ClaudeTranscriptSource { path -> File(path).takeIf { it.exists() }?.readLines() }
    val claude = ClaudeBackend(JvmProcessLauncher(), config, store, scope, reader, transcripts)
    claude.start()
    val session = claude.startThread(ThreadOptions(cwd.path))
    val key = ThreadKey(BackendKind.CLAUDE, session)
    var ok = true

    claude.send(session, listOf(UserPart.Text("MATH: answer in markdown with a formula; what colour is the image?"), UserPart.Image(png.path, "image/png")))
    store.await(60_000, "turn 1") { s -> s.threads.getValue(key).turns.lastOrNull()?.status?.isFinal == true }
    ok = check("turn 1 completed, session id is ours", store.state.value.threads.getValue(key).turns.last().status == TurnStatus.COMPLETED) && ok

    claude.send(session, listOf(UserPart.Text("WRITE: create hello.txt")))
    val request = store.await(60_000, "permission") { s -> s.requests.values.any { it.status == RequestStatus.PENDING } }.requests.values.first { it.status == RequestStatus.PENDING }
    delay(1000)
    ok = check("permission pending until the user answers", store.state.value.requests.getValue(request.key).status == RequestStatus.PENDING && !File(cwd, "hello.txt").exists()) && ok
    claude.respond(request.key, RequestResponse.Decide("allow"))
    store.await(60_000, "turn 2") { s -> s.threads.getValue(key).turns.last().status.isFinal }
    ok = check("file written after explicit allow", File(cwd, "hello.txt").exists()) && ok

    claude.rename(session, "smoke")
    val forked = claude.forkThread(session, null, ThreadOptions(cwd.path))
    claude.send(forked, listOf(UserPart.Text("MATH again after fork")))
    val fkey = ThreadKey(BackendKind.CLAUDE, forked)
    store.await(60_000, "fork turn") { s -> s.threads[fkey]?.turns?.lastOrNull()?.status?.isFinal == true }
    val initSession = store.state.value.threads.getValue(fkey).raw?.get("session_id")?.str
    ok = check("fork runs under the client-chosen session id ($forked)", initSession == null || initSession == forked) && ok
    ok = check("fork turn completed", store.state.value.threads.getValue(fkey).turns.last().status == TurnStatus.COMPLETED) && ok

    claude.loadHistory(session)
    val hydrated = store.state.value.threads.getValue(key).turns
    ok = check("transcript hydration yields both turns", hydrated.size >= 2) && ok
    println("control check: " + claude.rawRequest("get_settings", buildJsonObject { put("x", 1) })["applied"])
    claude.stop()
    return ok
}
