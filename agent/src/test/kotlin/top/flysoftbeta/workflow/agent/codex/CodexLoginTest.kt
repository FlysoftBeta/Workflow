package top.flysoftbeta.workflow.agent.codex

import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import kotlinx.serialization.json.*
import org.junit.After
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.agent.AgentStateStore
import top.flysoftbeta.workflow.agent.json.str
import top.flysoftbeta.workflow.agent.model.*
import top.flysoftbeta.workflow.agent.testing.*

class CodexLoginTest {
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private val store = AgentStateStore()
    private val authenticated = AtomicBoolean(false)
    private lateinit var process: FakeProcess
    private lateinit var backend: CodexBackend

    @After fun close() { if (::process.isInitialized) process.exit(0); scope.cancel() }

    private suspend fun start(timeout: Long = 5_000) {
        val launcher = FakeLauncher { _, spec ->
            FakeProcess(spec) { p, request ->
                val id = request["id"] ?: return@FakeProcess
                val result = when (request["method"].str) {
                    "account/read" -> if (authenticated.get())
                        """{"account":{"type":"chatgpt"},"requiresOpenaiAuth":true}"""
                    else """{"account":null,"requiresOpenaiAuth":true}"""
                    "account/login/start" -> """{"type":"chatgptDeviceCode","loginId":"test-login","verificationUrl":"https://example.test/device","userCode":"TEST-CODE"}"""
                    "model/list" -> """{"data":[],"nextCursor":null}"""
                    else -> "{}"
                }
                p.emit(buildJsonObject { put("id", id); put("result", Json.parseToJsonElement(result)) })
            }.also { process = it }
        }
        backend = CodexBackend(launcher, CodexConfig("codex", "/test/home"), store, scope,
            loginPollMillis = 20, loginTimeoutMillis = timeout)
        backend.start()
        store.await(what = "initial account") { it.backend(BackendKind.CODEX).account.state == LoginState.LOGGED_OUT }
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
    }

    private fun completion(id: String, success: Boolean) = process.emit(buildJsonObject {
        put("method", "account/login/completed")
        put("params", buildJsonObject { put("loginId", id); put("success", success); put("error", "test failure") })
    })

    @Test fun accountPollingRecoversMissingCompletionNotification() = runBlocking<Unit> {
        start()
        authenticated.set(true)
        store.await(what = "authenticated without notification") {
            val account = it.backend(BackendKind.CODEX).account
            account.state == LoginState.LOGGED_IN && account.login == null
        }
    }

    @Test fun timeoutEndsWaitingAndCancelsServerLogin() = runBlocking<Unit> {
        start(timeout = 100)
        store.await(what = "login timeout") {
            (it.backend(BackendKind.CODEX).account.login as? LoginFlow.Completed)?.error?.contains("超时") == true
        }
        withTimeout(2_000) {
            while (process.written.none { it["method"].str == "account/login/cancel" }) delay(10)
        }
        assertEquals(LoginState.LOGGED_OUT, store.state.value.backend(BackendKind.CODEX).account.state)
    }

    @Test fun staleCompletionCannotEndCurrentLogin() = runBlocking<Unit> {
        start()
        completion("older-login", false)
        delay(100)
        assertTrue(store.state.value.backend(BackendKind.CODEX).account.login is LoginFlow.DeviceCode)
        completion("test-login", false)
        store.await(what = "matching failure") {
            (it.backend(BackendKind.CODEX).account.login as? LoginFlow.Completed)?.error == "test failure"
        }
    }

    @Test fun cancellationAndProcessExitEndWaiting() = runBlocking<Unit> {
        start()
        backend.cancelLogin("test-login")
        assertEquals("cancelled", (store.state.value.backend(BackendKind.CODEX).account.login as LoginFlow.Completed).error)
        backend.login(LoginMethod.CODEX_DEVICE_CODE)
        process.exit(1)
        store.await(what = "login process exit") {
            val account = it.backend(BackendKind.CODEX).account
            account.login is LoginFlow.Completed && account.state != LoginState.LOGGING_IN
        }
    }
}
