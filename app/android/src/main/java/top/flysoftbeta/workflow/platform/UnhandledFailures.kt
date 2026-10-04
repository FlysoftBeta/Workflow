package top.flysoftbeta.workflow.platform

import android.util.Log
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.CoroutineExceptionHandler
import top.flysoftbeta.workflow.core.connection.Failure

/**
 * The last line of defence for coroutines (docs/app/workbench.md): a failure no caller handled is logged under
 * [TAG] and, when a UI owns the scope, reported there instead of terminating the process. Reaching it is still a
 * defect, so instrumentation tests assert that [defects] stays zero. A lost connection is counted separately
 * because the connection status already explains it.
 */
object UnhandledFailures {
    const val TAG = "Workflow"
    private val defectCount = AtomicInteger()
    private val lostCount = AtomicInteger()
    val defects: Int get() = defectCount.get()
    val lost: Int get() = lostCount.get()

    fun handler(owner: String, report: ((Failure) -> Unit)? = null) = CoroutineExceptionHandler { _, error ->
        val failure = Failure.of(error)
        if (failure is Failure.Lost) {
            lostCount.incrementAndGet()
            runCatching { Log.w(TAG, "Unhandled connection loss in $owner: ${error.message}") }
        } else {
            defectCount.incrementAndGet()
            runCatching { Log.e(TAG, "Unhandled failure in $owner", error) }
            report?.let { runCatching { it(failure) } }
        }
    }
}
