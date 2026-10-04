package top.flysoftbeta.workflow.platform.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import java.io.IOException
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.withTimeout

data class LocalRuntimeServiceState(
    val foreground: Boolean = false,
    val terminalCount: Int = 0,
    val codexCount: Int = 0,
    val proxyCount: Int = 0,
    /** Environment install/provision/reconcile work in progress (docs/engine/environment.md §5). */
    val environmentCount: Int = 0,
    val error: String? = null,
)

/** Independent from Activity/ViewModel and overlay lifetime. Only real process owners retain leases. */
class LocalRuntimeService : Service() {
    override fun onCreate() {
        super.onCreate()
        instance = this
        try {
            getSystemService(NotificationManager::class.java).createNotificationChannel(
                NotificationChannel(CHANNEL, "工作环境", NotificationManager.IMPORTANCE_LOW).apply {
                    description = "终端、对话与代理进程在后台运行时显示"; setShowBadge(false)
                }
            )
            acknowledgeStart()
        } catch (error: Exception) {
            mutableState.update { it.copy(foreground = false, error = error.message ?: "后台运行服务启动失败") }
            stopSelf()
        }
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // Android also requires acknowledgement when an existing Service receives a new FGS start.
        acknowledgeStart()
        return START_NOT_STICKY
    }
    private fun acknowledgeStart() = synchronized(controlLock) {
        val notification = notification()
        if (Build.VERSION.SDK_INT >= 34) startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        else startForeground(NOTIFICATION_ID, notification)
        startPending = false
        stopping = false
        mutableState.update { it.copy(foreground = true, error = null) }
        if (leases.isEmpty()) { stopping = true; stopSelf() }
    }
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onDestroy() {
        synchronized(controlLock) { if (instance === this) {
            instance = null; stopping = false
            mutableState.update { it.copy(foreground = false, error = if (leases.isEmpty() || startPending) it.error else it.error ?: "后台运行服务已停止，请回到应用重试") }
        } }
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }
    private fun notification(): Notification {
        val snapshot = snapshotCounts()
        val detail = buildList {
            if (snapshot.terminalCount > 0) add("${snapshot.terminalCount} 个终端")
            if (snapshot.codexCount > 0) add("Codex 运行中")
            if (snapshot.proxyCount > 0) add("代理运行中")
            if (snapshot.environmentCount > 0) add("工作区运行中")
        }.joinToString(" · ").ifEmpty { "正在准备工作环境" }
        val open = Intent().setClassName(packageName, "$packageName.MainActivity")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
            .putExtra("destination", "WORKBENCH")
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.ic_menu_manage)
            .setContentTitle("Workflow 工作环境")
            .setContentText(detail)
            .setContentIntent(PendingIntent.getActivity(this, NOTIFICATION_ID, open, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE))
            .setOngoing(true).setOnlyAlertOnce(true).build()
    }
    private fun updateNotification() {
        if (mutableState.value.foreground) getSystemService(NotificationManager::class.java).notify(NOTIFICATION_ID, notification())
    }

    companion object {
        private const val CHANNEL = "workflow_local_runtime"
        private const val NOTIFICATION_ID = 7420
        private enum class Owner { TERMINAL, CODEX, PROXY, ENVIRONMENT }
        private val leases = ConcurrentHashMap<String, Owner>()
        private val controlLock = Any()
        private var startPending = false
        private var stopping = false
        private val mutableState = MutableStateFlow(LocalRuntimeServiceState())
        val state: StateFlow<LocalRuntimeServiceState> = mutableState.asStateFlow()
        private val main = Handler(Looper.getMainLooper())
        @Volatile private var instance: LocalRuntimeService? = null

        /** Call during a user-initiated foreground operation. A background-start denial is returned to the caller. */
        suspend fun retainCodex(context: Context, processId: String): Result<Unit> = retain(context, "codex:$processId", Owner.CODEX)
        fun releaseCodex(context: Context, processId: String) = release(context, "codex:$processId")
        internal suspend fun retainProxy(context: Context, processId: String): Result<Unit> = retain(context, "proxy:$processId", Owner.PROXY)
        internal fun releaseProxy(context: Context, processId: String) = release(context, "proxy:$processId")
        internal suspend fun retainTerminal(context: Context, id: String): Result<Unit> = retain(context, "terminal:$id", Owner.TERMINAL)
        internal fun releaseTerminal(context: Context, id: String) = release(context, "terminal:$id")
        /** Environment work (image install, device provisioning, a declaration build) keeps running without the UI. */
        internal suspend fun retainEnvironment(context: Context, id: String): Result<Unit> = retain(context, "environment:$id", Owner.ENVIRONMENT)
        internal fun releaseEnvironment(context: Context, id: String) = release(context, "environment:$id")

        private suspend fun retain(context: Context, key: String, owner: Owner): Result<Unit> {
            return try {
                synchronized(controlLock) {
                    leases[key] = owner
                    refresh(context).getOrThrow()
                }
                val started = withTimeout(5000) { state.first { it.foreground || it.error != null } }
                check(started.foreground) { started.error ?: "后台运行服务未能启动" }
                Result.success(Unit)
            } catch (cancelled: CancellationException) {
                // A timeout is an observable service failure; ordinary caller cancellation still propagates.
                if (cancelled is kotlinx.coroutines.TimeoutCancellationException) {
                    val message = "后台运行服务未能及时启动"
                    mutableState.update { it.copy(error = message) }
                    Result.failure(IOException(message, cancelled))
                } else throw cancelled
            } catch (error: Exception) { Result.failure(error) }
        }

        /** Retains existing leases on failure: an already-created process is never secretly killed. */
        fun refresh(context: Context): Result<Unit> = runCatching { synchronized(controlLock) {
            snapshotCounts()
            if (leases.isEmpty()) {
                if (!startPending) { stopping = true; context.applicationContext.stopService(Intent(context, LocalRuntimeService::class.java)) }
                return@runCatching
            }
            mutableState.update { it.copy(error = null) }
            if (instance != null && mutableState.value.foreground && !stopping) {
                main.post { instance?.updateNotification() }
                return@runCatching
            }
            if (startPending) return@runCatching
            startPending = true
            mutableState.update { it.copy(foreground = false) }
            try { context.applicationContext.startForegroundService(Intent(context, LocalRuntimeService::class.java)) }
            catch (error: Exception) {
                startPending = false
                mutableState.update { it.copy(error = error.message ?: "系统不允许当前启动后台运行服务") }
                throw error
            }
            main.post { instance?.updateNotification() }
            Unit
        } }
        private fun release(context: Context, key: String) {
            synchronized(controlLock) {
                leases.remove(key)
                snapshotCounts()
                // Cancelling a start before startForeground is acknowledged crashes Android 9.
                // Let the pending service acknowledge it, then stop itself if no lease remains.
                if (leases.isEmpty() && !startPending) {
                    stopping = true
                    context.applicationContext.stopService(Intent(context, LocalRuntimeService::class.java))
                }
                else main.post { instance?.updateNotification() }
            }
        }
        private fun snapshotCounts(): LocalRuntimeServiceState {
            val counts = leases.values.toList()
            mutableState.update { it.copy(terminalCount = counts.count { owner -> owner == Owner.TERMINAL }, codexCount = counts.count { owner -> owner == Owner.CODEX }, proxyCount = counts.count { owner -> owner == Owner.PROXY }, environmentCount = counts.count { owner -> owner == Owner.ENVIRONMENT }) }
            return mutableState.value
        }
    }
}
