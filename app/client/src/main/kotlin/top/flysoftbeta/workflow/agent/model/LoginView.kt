package top.flysoftbeta.workflow.agent.model

/**
 * What a login surface shows for one backend. A pure projection of [AccountState] so conversation
 * panes, prompts and tests agree on the same phases (docs/ux/conversations.md "Signing in").
 */
sealed interface LoginPhase {
    /** The account has not been read yet and nothing failed. */
    data object Checking : LoginPhase

    /** Authenticated, including a completion the adapter is still confirming. */
    data object SignedIn : LoginPhase

    /** No login is running. [failure] is the last attempt's error; [checkError] a failed account read. */
    data class Idle(val failure: String?, val checkError: String?) : LoginPhase

    /** The start request is in flight; [attemptId] cancels it. */
    data class Starting(val attemptId: String) : LoginPhase

    /** Device code or browser flow waiting for the user; [checkError] reports a failing background check. */
    data class Waiting(val flow: LoginFlow, val checkError: String?) : LoginPhase

    /** Claude: complete the command in a terminal, then check again. */
    data class Terminal(val flow: LoginFlow.Terminal) : LoginPhase
}

object LoginView {
    /** Sentinel error of a login the user cancelled; never shown as a failure. */
    const val CANCELLED = "cancelled"

    /** Account re-reads after the app returns to the foreground while a login surface is visible. */
    val RESUME_RECHECK_DELAYS_MS: List<Long> = listOf(0, 2_000, 5_000)

    fun phase(account: AccountState): LoginPhase {
        val flow = account.login
        return when {
            account.state == LoginState.LOGGED_IN -> LoginPhase.SignedIn
            flow is LoginFlow.DeviceCode || flow is LoginFlow.Browser -> LoginPhase.Waiting(flow, account.checkError)
            flow is LoginFlow.Progress && flow.isPending -> LoginPhase.Starting(flow.loginId!!)
            flow is LoginFlow.Terminal -> LoginPhase.Terminal(flow)
            account.state == LoginState.UNKNOWN && account.checkError == null && failure(flow) == null -> LoginPhase.Checking
            else -> LoginPhase.Idle(failure(flow), account.checkError)
        }
    }

    /** True when the conversation must offer sign-in instead of the composer's normal state. */
    fun needsLogin(account: AccountState?): Boolean {
        account ?: return false
        val phase = phase(account)
        return phase !is LoginPhase.SignedIn && phase !is LoginPhase.Checking
    }

    /** True while [RESUME_RECHECK_DELAYS_MS] re-reads still have something to discover. */
    fun shouldRecheck(account: AccountState?): Boolean = account != null && account.state != LoginState.LOGGED_IN

    private fun failure(flow: LoginFlow?): String? = when (flow) {
        // An empty string is a failure without a message.
        is LoginFlow.Completed -> if (flow.success || flow.error == CANCELLED) null else flow.error.orEmpty()
        is LoginFlow.Progress -> flow.error
        else -> null
    }
}
