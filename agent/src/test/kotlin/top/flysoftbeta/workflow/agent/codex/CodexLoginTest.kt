package top.flysoftbeta.workflow.agent.codex

import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import org.junit.After
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.testing.*

/**
 * Login races replayed against the frame order the pinned Codex 0.157.1 app-server produced with a
 * local mock issuer (docs/engine/chat.md "Codex login state machine"): `account/login/completed` first, then
 * `account/updated`; a failed workspace-routing discovery reports `success:false` and makes
 * `account/read` answer with a JSON-RPC error; account requests are served one at a time.
 */
class CodexLoginTest {
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private val store = AgentStateStore()
    private val history = CopyOnWriteArrayList<AccountState>()
    private lateinit var process: FakeProcess
    private lateinit var backend: CodexBackend

    /** What the next `account/read` answers. */
    private sealed interface Read {
        data object Anonymous : Read
        data object Authenticated : Read
        data class Error(val message: String) : Read
        /** Not answered until [release]. */
        data object Hold : Read
    }

    @Volatile private var reads: (Int) -> Read = { Read.Anonymous }
    private val readCount = AtomicInteger()
    private val outstandingReads = AtomicInteger()
    private val maxOutstandingReads = AtomicInteger()
    private val held = ConcurrentLinkedQueue<JsonElement>()
    @Volatile private var holdStart = false
    private var heldStart: JsonElement? = null
    @Volatile private var startError: String? = null
    private val loginIds = AtomicInteger()

    init {
        store.addListener { if (it is AgentEvent.AccountChanged || it is AgentEvent.LoginChanged || it is AgentEvent.AccountCheckFailed) history += store.state.value.backend(BackendKind.CODEX).account }
    }

    @After fun close() { if (::process.isInitialized) process.exit(0); scope.cancel() }

    private fun answer(p: FakeProcess, id: JsonElement, result: String) =
        p.emit(buildJsonObject { put("id", id); put("result", Json.parseToJsonElement(result)) })

    private fun answerRead(p: FakeProcess, id: JsonElement, read: Read) {
        when (read) {
            Read.Anonymous -> answer(p, id, """{"account":null,"requiresOpenaiAuth":true,"workspaceRouting":null}""")
            Read.Authenticated -> answer(p, id, """{"account":{"type":"chatgpt","email":"tester@example.test","planType":"plus"},"requiresOpenaiAuth":true,"workspaceRouting":null}""")
            is Read.Error -> p.emit(buildJsonObject { put("id", id); put("error", buildJsonObject { put("code", -32603); put("message", read.message) }) })
            Read.Hold -> { held += id; return }
        }
        outstandingReads.decrementAndGet()
    }

    private fun release(read: Read) {
        while (true) answerRead(process, held.poll() ?: return, read)
    }

    private suspend fun start(
        timeout: Long = 5_000,
        poll: Long = 20,
        readTimeout: Long = 2_000,
        confirmDelays: List<Long> = listOf(0, 20, 40),
    ) {
        val launcher = FakeLauncher { _, spec ->
            FakeProcess(spec) { p, request ->
                val id = request["id"] ?: return@FakeProcess
                when (request["method"].str) {
                    "account/read" -> {
                        val now = outstandingReads.incrementAndGet()
                        maxOutstandingReads.accumulateAndGet(now, ::maxOf)
                        answerRead(p, id, reads(readCount.incrementAndGet()))
                    }
                    "account/login/start" -> {
                        val loginId = "login-${loginIds.incrementAndGet()}"
                        val type = request["params"]?.jsonObject?.get("type").str
                        val result = if (type == "chatgpt") """{"type":"chatgpt","loginId":"$loginId","authUrl":"https://auth.example.test/oauth/authorize"}"""
                        else """{"type":"chatgptDeviceCode","loginId":"$loginId","verificationUrl":"https://auth.example.test/codex/device","userCode":"TEST-0000"}"""
                        when {
                            startError != null -> p.emit(buildJsonObject { put("id", id); put("error", buildJsonObject { put("code", -32603); put("message", startError) }) })
                            holdStart -> { heldStart = id; pendingStartResult = result }
                            else -> answer(p, id, result)
                        }
                    }
                    "account/login/cancel" -> answer(p, id, """{"status":"canceled"}""")
                    "account/logout" -> answer(p, id, "{}")
                    "model/list" -> answer(p, id, """{"data":[],"nextCursor":null}""")
                    else -> answer(p, id, "{}")
                }
            }.also { process = it }
        }
        backend = CodexBackend(launcher, CodexConfig("codex", "/test/home"), store, scope,
            loginPollMillis = poll, loginTimeoutMillis = timeout, accountReadTimeoutMillis = readTimeout,
            maxLoginPollMillis = poll * 4, confirmDelaysMillis = confirmDelays)
        backend.start()
        store.await(what = "initial account") { it.account().state == LoginState.LOGGED_OUT }
    }

    @Volatile private var pendingStartResult: String? = null

    private fun AgentState.account() = backend(BackendKind.CODEX).account
    private fun account() = store.state.value.account()

    private fun notify(method: String, params: String) = process.emit(buildJsonObject {
        put("method", method); put("params", Json.parseToJsonElement(params))
    })

    private fun completed(loginId: String?, success: Boolean, error: String? = null) = notify("account/login/completed",
        buildJsonObject { put("loginId", loginId); put("success", success); put("error", error); put("onboardingEntrypoint", null) }.toString())

    private fun updated() = notify("account/updated", """{"authMode":"chatgpt","planType":"plus"}""")

    private fun written(method: String) = process.written.count { it["method"].str == method }

    private suspend fun awaitWritten(method: String, count: Int = 1) = withTimeout(5_000) {
        while (written(method) < count) delay(5)
    }

    @Test fun realDeviceCodeSequenceEndsLoggedIn() = runBlocking<Unit> {
        start(poll = 60_000)
        val flow = backend.login(LoginMethod.CODEX_DEVICE_CODE)
        assertEquals("login-1", flow.loginId)
        assertTrue(account().login is LoginFlow.DeviceCode)
        assertEquals(LoginState.LOGGING_IN, account().state)
        reads = { Read.Authenticated }
        completed("login-1", true)
        updated()
        val done = store.await(what = "confirmed account") { it.account().state == LoginState.LOGGED_IN && it.account().login == null }
        assertEquals("tester@example.test", done.account().email)
        assertTrue("the start shows a pending attempt first", history.any { it.login is LoginFlow.Progress && it.state == LoginState.LOGGING_IN })
    }

    @Test fun nullReadsAfterCompletionNeverResetToLoggedOut() = runBlocking<Unit> {
        start(poll = 60_000)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        val base = readCount.get()
        // Codex has not reloaded its credentials yet for the first two reads after completion.
        reads = { n -> if (n <= base + 2) Read.Anonymous else Read.Authenticated }
        completed("login-1", true)
        updated()
        store.await(what = "confirmed account") { it.account().email != null }
        val afterCompletion = history.dropWhile { !(it.login is LoginFlow.Completed && (it.login as LoginFlow.Completed).success) }
        assertTrue(afterCompletion.isNotEmpty())
        assertTrue("a null read overrode the completion: $afterCompletion", afterCompletion.all { it.state == LoginState.LOGGED_IN })
    }

    @Test fun completionThatNeverReadsBackStaysLoggedIn() = runBlocking<Unit> {
        start(poll = 60_000)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        completed("login-1", true)
        val notice = store.await(what = "unconfirmed notice") { s -> s.backend(BackendKind.CODEX).notices.any { it.code == "loginAccountUnconfirmed" } }
        assertEquals(LoginState.LOGGED_IN, notice.account().state)
    }

    @Test fun pollThatAuthenticatesBeforeTheNotificationWins() = runBlocking<Unit> {
        start()
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        reads = { Read.Authenticated }
        store.await(what = "authenticated by polling") { it.account().state == LoginState.LOGGED_IN && it.account().login == null }
        completed("login-1", true) // late notification for an attempt the poll already finished
        updated()
        delay(150)
        assertEquals(LoginState.LOGGED_IN, account().state)
        assertNull(account().login)
    }

    @Test fun unattributedSuccessIsConfirmedByAnAccountRead() = runBlocking<Unit> {
        start(poll = 60_000)
        backend.login(LoginMethod.CODEX_BROWSER)
        assertTrue(account().login is LoginFlow.Browser)
        reads = { Read.Authenticated }
        completed(null, true)
        store.await(what = "confirmed by read") { it.account().state == LoginState.LOGGED_IN && it.account().login == null }
    }

    @Test fun staleCompletionCannotEndCurrentLogin() = runBlocking<Unit> {
        start()
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        completed("older-login", false, "old failure")
        delay(100)
        assertTrue(account().login is LoginFlow.DeviceCode)
        completed("login-1", false, "test failure")
        store.await(what = "matching failure") { (it.account().login as? LoginFlow.Completed)?.error == "test failure" }
        assertEquals(LoginState.LOGGED_OUT, account().state)
    }

    @Test fun routingDiscoveryFailureShowsTheReasonAndTheAccountError() = runBlocking<Unit> {
        start(poll = 60_000)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        // Real 0.157.1 behaviour: credentials were stored, discovery failed, every read now errors.
        reads = { Read.Error("workspace routing discovery failed") }
        completed("login-1", false, "workspace routing discovery failed")
        val failed = store.await(what = "account error") { it.account().checkError != null }
        assertEquals(LoginState.LOGGED_OUT, failed.account().state)
        assertEquals("workspace routing discovery failed", (failed.account().login as LoginFlow.Completed).error)
        assertEquals("workspace routing discovery failed", failed.account().checkError)
        assertEquals(LoginPhase.Idle("workspace routing discovery failed", "workspace routing discovery failed"), LoginView.phase(failed.account()))
        // Later, with the network back, Check again signs in without a new login.
        reads = { Read.Authenticated }
        backend.refreshAccount()
        store.await(what = "recovered") { it.account().state == LoginState.LOGGED_IN && it.account().checkError == null && it.account().login == null }
    }

    @Test fun slowReadsStaySingleAndDoNotDelayCancellation() = runBlocking<Unit> {
        start(poll = 10, readTimeout = 100)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        // After the browser step Codex waits up to 15 s in routing discovery for each read.
        reads = { Read.Hold }
        store.await(what = "check failure reported") { it.account().checkError != null }
        repeat(3) { backend.refreshAccount() } // resume re-checks join the outstanding read
        delay(200)
        assertEquals("one account/read outstanding at most", 1, maxOutstandingReads.get())
        assertEquals(LoginState.LOGGING_IN, account().state)
        assertTrue(LoginView.phase(account()) is LoginPhase.Waiting)
        backend.cancelLogin("login-1")
        assertEquals(LoginView.CANCELLED, (account().login as LoginFlow.Completed).error)
        assertEquals(LoginState.LOGGED_OUT, account().state)
        awaitWritten("account/login/cancel")
        release(Read.Error("workspace routing discovery timed out"))
        delay(100)
        assertEquals(LoginState.LOGGED_OUT, account().state)
    }

    @Test fun cancellationWhileStartingReleasesTheServerLogin() = runBlocking<Unit> {
        start()
        holdStart = true
        val login = async { backend.login(LoginMethod.CODEX_DEVICE_CODE) }
        val starting = store.await(what = "starting") { it.account().login is LoginFlow.Progress }
        val phase = LoginView.phase(starting.account()) as LoginPhase.Starting
        backend.cancelLogin(phase.attemptId)
        assertEquals(LoginView.CANCELLED, (withTimeout(2_000) { login.await() } as LoginFlow.Completed).error)
        assertEquals(LoginState.LOGGED_OUT, account().state)
        // The start answer arrives afterwards; the adapter cancels the login it created.
        answer(process, heldStart!!, pendingStartResult!!)
        awaitWritten("account/login/cancel")
        val cancel = process.written.last { it["method"].str == "account/login/cancel" }
        assertEquals("login-1", cancel["params"]!!.jsonObject["loginId"].str)
        assertTrue(account().login is LoginFlow.Completed)
    }

    @Test fun startFailureIsReturnedAsVisibleFailure() = runBlocking<Unit> {
        start()
        startError = "failed to request device code"
        val flow = backend.login(LoginMethod.CODEX_DEVICE_CODE) as LoginFlow.Completed
        assertFalse(flow.success)
        assertEquals("failed to request device code", flow.error)
        assertEquals(LoginPhase.Idle("failed to request device code", null), LoginView.phase(account()))
    }

    @Test fun resumeReadFinishesAWaitingLogin() = runBlocking<Unit> {
        start(poll = 60_000)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        reads = { Read.Authenticated }
        backend.refreshAccount() // what the app does when it returns from the browser
        assertEquals(LoginState.LOGGED_IN, account().state)
        assertNull(account().login)
        val count = readCount.get()
        delay(150)
        assertEquals("no polling after the attempt ended", count, readCount.get())
    }

    @Test fun accountPollingRecoversMissingCompletionNotification() = runBlocking<Unit> {
        start()
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        reads = { Read.Authenticated }
        store.await(what = "authenticated without notification") { it.account().state == LoginState.LOGGED_IN && it.account().login == null }
    }

    @Test fun timeoutEndsWaitingAndCancelsServerLogin() = runBlocking<Unit> {
        start(timeout = 100)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        store.await(what = "login timeout") { (it.account().login as? LoginFlow.Completed)?.error?.contains("超时") == true }
        awaitWritten("account/login/cancel")
        assertEquals(LoginState.LOGGED_OUT, account().state)
    }

    @Test fun cancellationAndProcessExitEndWaiting() = runBlocking<Unit> {
        start()
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        backend.cancelLogin("login-1")
        assertEquals(LoginView.CANCELLED, (account().login as LoginFlow.Completed).error)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        process.exit(1)
        store.await(what = "login process exit") { it.account().login is LoginFlow.Completed && it.account().state != LoginState.LOGGING_IN }
    }

    @Test fun signOutNotificationLetsTheLoggedOutReadApply() = runBlocking<Unit> {
        start(poll = 60_000)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        completed("login-1", true)
        store.await(what = "unconfirmed") { s -> s.backend(BackendKind.CODEX).notices.any { it.code == "loginAccountUnconfirmed" } }
        notify("account/updated", """{"authMode":null,"planType":null}""")
        store.await(what = "signed out") { it.account().state == LoginState.LOGGED_OUT && it.account().login == null }
    }

    @Test fun startupReadFailureIsVisibleInsteadOfUnknown() = runBlocking<Unit> {
        reads = { Read.Error("workspace routing discovery timed out") }
        val launcher = FakeLauncher { _, spec ->
            FakeProcess(spec) { p, request ->
                val id = request["id"] ?: return@FakeProcess
                if (request["method"].str == "account/read") { outstandingReads.incrementAndGet(); answerRead(p, id, reads(readCount.incrementAndGet())) }
                else answer(p, id, if (request["method"].str == "model/list") """{"data":[],"nextCursor":null}""" else "{}")
            }.also { process = it }
        }
        backend = CodexBackend(launcher, CodexConfig("codex", "/test/home"), store, scope)
        backend.start()
        val failed = store.await(what = "startup check failure") { it.account().checkError != null }
        assertEquals(LoginState.UNKNOWN, failed.account().state)
        assertTrue(LoginView.needsLogin(failed.account()))
        assertEquals(LoginPhase.Idle(null, "workspace routing discovery timed out"), LoginView.phase(failed.account()))
    }
}
