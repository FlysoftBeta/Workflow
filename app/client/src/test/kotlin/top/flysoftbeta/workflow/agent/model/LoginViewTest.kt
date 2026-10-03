package top.flysoftbeta.workflow.agent.model

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import top.flysoftbeta.workflow.agent.rpc.ChatWire

class LoginViewTest {
    private val device = LoginFlow.DeviceCode("l1", "https://auth.example.test/device", "TEST-0000")

    @Test fun phasesFollowTheLoginStateMachine() {
        assertEquals(LoginPhase.Checking, LoginView.phase(AccountState()))
        assertEquals(LoginPhase.Idle(null, null), LoginView.phase(AccountState(LoginState.LOGGED_OUT)))
        assertEquals(LoginPhase.Starting("attempt:1"), LoginView.phase(AccountState(LoginState.LOGGING_IN, login = LoginFlow.Progress("attempt:1", emptyList()))))
        assertEquals(LoginPhase.Waiting(device, "slow"), LoginView.phase(AccountState(LoginState.LOGGING_IN, login = device, checkError = "slow")))
        assertEquals(LoginPhase.SignedIn, LoginView.phase(AccountState(LoginState.LOGGED_IN, login = LoginFlow.Completed("l1", true, null))))
        assertEquals(LoginPhase.Idle("denied", null), LoginView.phase(AccountState(LoginState.LOGGED_OUT, login = LoginFlow.Completed("l1", false, "denied"))))
        assertEquals(LoginPhase.Idle("", null), LoginView.phase(AccountState(LoginState.LOGGED_OUT, login = LoginFlow.Completed("l1", false, null))))
    }

    @Test fun cancellationIsNotAFailure() {
        val cancelled = AccountState(LoginState.LOGGED_OUT, login = LoginFlow.Completed("l1", false, LoginView.CANCELLED))
        assertEquals(LoginPhase.Idle(null, null), LoginView.phase(cancelled))
    }

    @Test fun unreadableAccountNeedsAVisibleSurface() {
        val unknown = AccountState(checkError = "workspace routing discovery timed out")
        assertEquals(LoginPhase.Idle(null, "workspace routing discovery timed out"), LoginView.phase(unknown))
        assertTrue(LoginView.needsLogin(unknown))
        assertFalse(LoginView.needsLogin(AccountState()))
        assertFalse(LoginView.needsLogin(null))
        assertFalse(LoginView.needsLogin(AccountState(LoginState.LOGGED_IN, checkError = "background refresh failed")))
    }

    @Test fun aRunningLoginKeepsTheLoginSurface() {
        assertTrue(LoginView.needsLogin(AccountState(LoginState.LOGGING_IN, login = device)))
        assertTrue(LoginView.needsLogin(AccountState(LoginState.LOGGING_IN, login = LoginFlow.Progress("attempt:1", emptyList()))))
    }

    @Test fun claudeStatusProgressIsNotAnAttempt() {
        assertFalse(LoginFlow.Progress(null, listOf("Opening browser")).isPending)
        assertTrue(LoginFlow.Progress("attempt:1", emptyList()).isPending)
        assertFalse(LoginFlow.Progress("attempt:1", emptyList(), error = "failed").isPending)
    }

    @Test fun resumeRechecksAreBoundedAndStopWhenSignedIn() {
        assertEquals(3, LoginView.RESUME_RECHECK_DELAYS_MS.size)
        assertTrue(LoginView.RESUME_RECHECK_DELAYS_MS.sum() <= 10_000)
        assertTrue(LoginView.shouldRecheck(AccountState(LoginState.LOGGING_IN, login = device)))
        assertFalse(LoginView.shouldRecheck(AccountState(LoginState.LOGGED_IN)))
        assertFalse(LoginView.shouldRecheck(null))
    }

    @Test fun accountWithoutCheckErrorDecodes() {
        val decoded = ChatWire.decode<AccountState>(ChatWire.json.parseToJsonElement("""{"state":"LOGGED_OUT"}"""))
        assertEquals(null, decoded.checkError)
        assertEquals("x", ChatWire.decode<AccountState>(ChatWire.encode(AccountState(checkError = "x"))).checkError)
    }
}
