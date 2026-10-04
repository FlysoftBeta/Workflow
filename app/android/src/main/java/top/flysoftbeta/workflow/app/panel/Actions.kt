package top.flysoftbeta.workflow.app.panel

import android.util.Log
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import top.flysoftbeta.workflow.core.connection.Failure
import top.flysoftbeta.workflow.platform.UnhandledFailures

/**
 * Runs a UI-initiated operation (docs/ux/design-system.md, "Readiness and failures"). A lost connection ends it
 * without a report because the connection card already explains it; any other failure is logged and goes to
 * [report] as a classified [Failure], never to the thread.
 */
fun CoroutineScope.launchAction(report: (Failure) -> Unit, block: suspend CoroutineScope.() -> Unit): Job = launch {
    try { block() }
    catch (cancelled: CancellationException) { throw cancelled }
    catch (error: Exception) {
        val failure = Failure.of(error)
        if (failure is Failure.Lost) return@launch
        runCatching { Log.w(UnhandledFailures.TAG, "Action failed: ${failure.summary} (${failure.detail})") }
        report(failure)
    }
}

/** [launchAction] for an action without a local error row: the Workbench Snackbar reports the failure. */
fun CoroutineScope.launchAction(commands: WorkbenchCommands, block: suspend CoroutineScope.() -> Unit): Job =
    launchAction({ commands.snackbar(it.summary) }, block)
