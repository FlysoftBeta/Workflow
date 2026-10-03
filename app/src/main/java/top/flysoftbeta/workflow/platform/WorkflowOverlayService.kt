package top.flysoftbeta.workflow.platform

import android.animation.ValueAnimator
import android.app.*
import android.content.*
import android.content.pm.ServiceInfo
import android.content.res.Configuration
import android.graphics.PixelFormat
import android.graphics.Point
import android.graphics.drawable.GradientDrawable
import android.os.*
import android.view.*
import android.widget.FrameLayout
import android.widget.ImageView
import androidx.core.content.ContextCompat
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.lifecycle.*
import androidx.savedstate.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import org.json.JSONObject
import top.flysoftbeta.workflow.R
import top.flysoftbeta.workflow.app.AppGraph
import top.flysoftbeta.workflow.core.config.ThemeMode
import top.flysoftbeta.workflow.platform.clientservices.WorkspaceDocuments
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager
import top.flysoftbeta.workflow.platform.connection.ConnectionStatus
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.roundToInt

/** Explicitly enabled, process-independent overlay. All persistent IO runs on Dispatchers.IO. */
class WorkflowOverlayService : Service(), LifecycleOwner, SavedStateRegistryOwner {
    private val registry = LifecycleRegistry(this)
    private val saved = SavedStateRegistryController.create(this)
    override val lifecycle: Lifecycle get() = registry
    override val savedStateRegistry: SavedStateRegistry get() = saved.savedStateRegistry
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private lateinit var windows: WindowManager
    private lateinit var capabilities: DeviceCapabilities
    private val main = Handler(Looper.getMainLooper())
    private var dock: FrameLayout? = null
    private var glyph: ImageView? = null
    private var badge: View? = null
    private var expanded: FrameLayout? = null
    private var dockParams: WindowManager.LayoutParams? = null
    private var animation: ValueAnimator? = null
    private var receiverRegistered = false
    private var onRight = true
    private var verticalFraction = .42f
    private var touchedPosition = false
    private val screen = Point()
    private lateinit var documents: WorkspaceDocuments
    private lateinit var sessionStore: top.flysoftbeta.workflow.core.store.WorkspaceStore
    private val dim = Runnable { dock?.animate()?.alpha(.45f)?.setDuration(180)?.start() }
    private val locked get() = getSystemService(KeyguardManager::class.java).isKeyguardLocked || !getSystemService(PowerManager::class.java).isInteractive
    private val overlayPermissionListener = AppOpsManager.OnOpChangedListener { _, pkg ->
        if (pkg == packageName && !android.provider.Settings.canDrawOverlays(this)) main.post { stopSelf() }
    }
    private val screenReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            when (intent?.action) {
                Intent.ACTION_SCREEN_OFF -> { collapse(); dock?.visibility = View.GONE }
                Intent.ACTION_CLOSE_SYSTEM_DIALOGS -> collapse()
                else -> dock?.visibility = if (locked) View.GONE else View.VISIBLE
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        saved.performAttach(); saved.performRestore(null)
        registry.handleLifecycleEvent(Lifecycle.Event.ON_CREATE)
        registry.handleLifecycleEvent(Lifecycle.Event.ON_START)
        registry.handleLifecycleEvent(Lifecycle.Event.ON_RESUME)
        capabilities = DeviceCapabilities(this)
        windows = getSystemService(WindowManager::class.java)
        if (!android.provider.Settings.canDrawOverlays(this)) { stopSelf(); return }
        try {
            val session = WorkspaceConnectionManager.get(this).requireSession()
            documents = WorkspaceDocuments(session.rpc)
            sessionStore = session.store
            createNotification(); updateScreenSize(); showDock(); isRunning = true
            scope.launch {
                WorkspaceConnectionManager.get(this@WorkflowOverlayService).status.collect { status ->
                    if (status !is ConnectionStatus.Connected || status.session.token != session.token) stopSelf()
                }
            }
            val filter = IntentFilter().apply {
                addAction(Intent.ACTION_SCREEN_OFF); addAction(Intent.ACTION_SCREEN_ON)
                addAction(Intent.ACTION_USER_PRESENT); addAction(Intent.ACTION_CLOSE_SYSTEM_DIALOGS)
            }
            // These broadcasts come from the system UID; no other app may dismiss our controls.
            ContextCompat.registerReceiver(this, screenReceiver, filter, ContextCompat.RECEIVER_NOT_EXPORTED)
            receiverRegistered = true
            getSystemService(AppOpsManager::class.java).startWatchingMode(AppOpsManager.OPSTR_SYSTEM_ALERT_WINDOW, packageName, overlayPermissionListener)
            scope.launch {
                try {
                    val stored = documents.read("overlay", "position").text?.let(::JSONObject)
                    if (stored != null && !touchedPosition) {
                        onRight = stored.optBoolean("right", true)
                        verticalFraction = OverlayPosition.safeFraction(stored.optDouble("verticalFraction", .42).toFloat())
                        placeDock()
                    }
                } catch (cancelled: CancellationException) { throw cancelled }
                catch (error: Exception) { showError(error) }
            }
            scope.launch {
                val store = sessionStore
                store.awaitReady()
                store.state.map { it.config }.distinctUntilChanged().collect { config ->
                    if (!config.overlay.enabled) stopSelf() else colorDock(config.appearance.theme)
                }
            }
            scope.launch {
                AppGraph.agentHub(this@WorkflowOverlayService).attention.collect { needsAttention -> badge?.visibility = if (needsAttention) View.VISIBLE else View.GONE }
            }
        } catch (error: Exception) { showError(error); stopSelf() }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                AppGraph.processScope.launch { sessionStore.updateConfig { it.copy(overlay = it.overlay.copy(enabled = false)) } }
                stopSelf()
            }
            ACTION_RESET -> { touchedPosition = true; onRight = true; verticalFraction = .42f; collapse(); placeDock(); savePosition() }
        }
        return START_NOT_STICKY
    }
    override fun onBind(intent: Intent?): IBinder? = null

    private fun createNotification() {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(CHANNEL, "悬浮快捷控制", NotificationManager.IMPORTANCE_LOW))
        val open = PendingIntent.getActivity(this, 7410, destinationIntent("SETTINGS"), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        val stop = PendingIntent.getService(this, 7411, Intent(this, WorkflowOverlayService::class.java).setAction(ACTION_STOP), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        val notification = Notification.Builder(this, CHANNEL).setSmallIcon(R.drawable.ic_workflow_glyph)
            .setContentTitle("Workflow 悬浮窗").setContentText("点按管理快捷控制").setContentIntent(open)
            .setOngoing(true).setOnlyAlertOnce(true).addAction(Notification.Action.Builder(null, "关闭悬浮窗", stop).build()).build()
        if (Build.VERSION.SDK_INT >= 34) startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        else startForeground(NOTIFICATION_ID, notification)
    }

    private fun showDock() {
        val bubble = FrameLayout(this).apply {
            elevation = dp(4).toFloat(); contentDescription = "Workflow 快捷控制，点按展开，拖动贴边"
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
            setOnClickListener { if (expanded == null) expand() else collapse() }
        }
        glyph = ImageView(this).apply { setImageResource(R.drawable.ic_workflow_glyph); setPadding(dp(10), dp(10), dp(10), dp(10)) }
        bubble.addView(glyph, FrameLayout.LayoutParams(dp(44), dp(44), Gravity.CENTER))
        badge = View(this).apply { visibility = View.GONE; background = rounded(0xff775a12.toInt(), 4) }
        bubble.addView(badge, FrameLayout.LayoutParams(dp(8), dp(8), Gravity.TOP or Gravity.CENTER_HORIZONTAL).apply { topMargin = dp(2) })
        val params = WindowManager.LayoutParams(dp(48), dp(48), WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL or WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT).apply { gravity = Gravity.TOP or Gravity.LEFT }
        dock = bubble; dockParams = params
        colorDock(ThemeMode.SYSTEM)
        bubble.setOnTouchListener(object : View.OnTouchListener {
            var startX = 0; var startY = 0; var touchX = 0f; var touchY = 0f; var dragging = false
            val threshold = ViewConfiguration.get(this@WorkflowOverlayService).scaledTouchSlop
            override fun onTouch(view: View, event: MotionEvent): Boolean {
                when (event.actionMasked) {
                    MotionEvent.ACTION_DOWN -> {
                        main.removeCallbacks(dim); view.animate().cancel(); view.alpha = 1f
                        animation?.cancel(); startX = params.x; startY = params.y; touchX = event.rawX; touchY = event.rawY; dragging = false
                    }
                    MotionEvent.ACTION_MOVE -> {
                        val dx = event.rawX - touchX; val dy = event.rawY - touchY
                        if (abs(dx) > threshold || abs(dy) > threshold) dragging = true
                        if (dragging) {
                            touchedPosition = true; view.scaleX = 1.1f; view.scaleY = 1.1f
                            params.x = (startX + dx.roundToInt()).coerceIn(-dp(22), (screen.x - dp(26)).coerceAtLeast(0))
                            params.y = (startY + dy.roundToInt()).coerceIn(dp(24), (screen.y - dp(96)).coerceAtLeast(dp(24)))
                            runCatching { windows.updateViewLayout(view, params) }
                        }
                    }
                    MotionEvent.ACTION_UP -> { view.scaleX = 1f; view.scaleY = 1f; if (dragging) snapToEdge() else view.performClick(); scheduleDim() }
                    MotionEvent.ACTION_CANCEL -> { view.scaleX = 1f; view.scaleY = 1f; snapToEdge(); scheduleDim() }
                }
                return true
            }
        })
        placeDock(false); windows.addView(bubble, params); bubble.visibility = if (locked) View.GONE else View.VISIBLE; scheduleDim()
    }

    private fun colorDock(theme: ThemeMode) {
        val dark = theme == ThemeMode.DARK || theme == ThemeMode.SYSTEM && resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
        glyph?.setColorFilter(if (dark) 0xffa3d3b8.toInt() else 0xff286b57.toInt())
        glyph?.background = rounded(if (dark) 0xff353d38.toInt() else 0xffdde5df.toInt(), 22)
    }
    private fun dockX() = if (onRight) (screen.x - dp(26)).coerceAtLeast(0) else -dp(22)
    private fun placeDock(update: Boolean = true) {
        val params = dockParams ?: return
        params.x = dockX(); params.y = (dp(24) + (screen.y - dp(120)).coerceAtLeast(0) * verticalFraction).roundToInt()
        if (update) dock?.let { runCatching { windows.updateViewLayout(it, params) } }
    }
    private fun snapToEdge() {
        val params = dockParams ?: return
        onRight = params.x + dp(24) >= screen.x / 2
        verticalFraction = OverlayPosition.safeFraction((params.y - dp(24)).toFloat() / (screen.y - dp(120)).coerceAtLeast(1))
        animation = ValueAnimator.ofInt(params.x, dockX()).apply {
            duration = 280
            // Critically damped spatial spring, normalized to finish exactly on the chosen edge.
            setInterpolator { t -> ((1 - (1 + 8 * t) * exp(-8 * t)) / (1 - 9 * exp(-8f))).coerceIn(0f, 1f) }
            addUpdateListener { params.x = it.animatedValue as Int; dock?.let { view -> runCatching { windows.updateViewLayout(view, params) } } }
            start()
        }
        savePosition()
    }
    private fun savePosition() {
        val right = onRight; val fraction = verticalFraction
        AppGraph.processScope.launch(positionIo) { runCatching { writePosition(documents, right, fraction) }.onFailure(::showError) }
    }
    private fun scheduleDim() { main.removeCallbacks(dim); main.postDelayed(dim, 3_000) }

    private fun expand(initialError: String? = null) {
        if (locked) return
        if (!android.provider.Settings.canDrawOverlays(this)) { stopSelf(); return }
        val shade = object : FrameLayout(this) {
            override fun dispatchKeyEvent(event: KeyEvent): Boolean {
                if (event.keyCode == KeyEvent.KEYCODE_BACK) {
                    if (event.action == KeyEvent.ACTION_UP) collapse()
                    return true
                }
                return super.dispatchKeyEvent(event)
            }
        }.apply {
            setBackgroundColor(0x22000000); isFocusableInTouchMode = true
            setOnClickListener { collapse() }
            setViewTreeLifecycleOwner(this@WorkflowOverlayService); setViewTreeSavedStateRegistryOwner(this@WorkflowOverlayService)
        }
        val card = ComposeView(this).apply {
            setOnClickListener { }
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent { OverlayPanel(capabilities, initialError = initialError, onManageApps = {
                SettingsEntryPoint.destination.value = SettingsEntryPoint.Destination.OVERLAY_APPS
                collapse(); launchDestination("SETTINGS")
            }, onPermissions = {
                SettingsEntryPoint.destination.value = SettingsEntryPoint.Destination.PERMISSIONS
                collapse(); launchDestination("SETTINGS")
            }, onLaunch = { destination -> collapse(); launchDestination(destination) }, onClose = ::collapse, onLock = {
                collapse()
                capabilities.lockScreen { result ->
                    result.onFailure { failure ->
                        // A failed lock leaves the screen interactive: restore the panel so the real
                        // failure stays at the control that caused it, including after composition disposal.
                        if (!locked && dock != null) expand(failure.message ?: "锁屏未完成")
                        else showError(failure)
                    }
                }
            }) }
        }
        val frame = FrameLayout.LayoutParams(dp(320).coerceAtMost(screen.x - dp(24)), FrameLayout.LayoutParams.WRAP_CONTENT).apply {
            gravity = Gravity.TOP or if (onRight) Gravity.RIGHT else Gravity.LEFT
            leftMargin = dp(12); rightMargin = dp(12); topMargin = dp(24)
        }
        shade.addView(card, frame)
        val params = WindowManager.LayoutParams(-1, -1, WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN, PixelFormat.TRANSLUCENT).apply { gravity = Gravity.TOP or Gravity.LEFT }
        try {
            windows.addView(shade, params); expanded = shade; dock?.visibility = View.INVISIBLE; shade.requestFocus()
            card.post {
                frame.topMargin = ((dockParams?.y ?: dp(80)) + dp(24) - card.height / 2).coerceIn(dp(24), (shade.height - card.height - dp(24)).coerceAtLeast(dp(24)))
                card.layoutParams = frame
                card.pivotX = if (onRight) card.width.toFloat() else 0f; card.pivotY = card.height / 2f
                card.alpha = 0f; card.scaleX = .85f; card.scaleY = .85f; card.animate().alpha(1f).scaleX(1f).scaleY(1f).setDuration(180).start()
            }
        } catch (error: Exception) { showError(error); stopSelf() }
    }
    private fun collapse() {
        expanded?.let { runCatching { windows.removeView(it) } }; expanded = null
        dock?.visibility = if (locked) View.GONE else View.VISIBLE; scheduleDim()
    }
    private fun rounded(color: Int, radius: Int) = GradientDrawable().apply { setColor(color); cornerRadius = dp(radius).toFloat() }
    private fun dp(value: Int) = (value * resources.displayMetrics.density).roundToInt()
    private fun destinationIntent(destination: String) = Intent().setClassName(packageName, "$packageName.MainActivity")
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP).putExtra("destination", destination)
    private fun launchDestination(destination: String) { runCatching { startActivity(destinationIntent(destination)) }.onFailure(::showError) }
    private fun showError(error: Throwable) { failureState.value = error.message ?: "操作未完成" }
    @Suppress("DEPRECATION") private fun updateScreenSize() { windows.defaultDisplay.getSize(screen) }
    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig); collapse(); updateScreenSize(); placeDock()
        if (::sessionStore.isInitialized) colorDock(sessionStore.state.value.config.appearance.theme)
    }
    override fun onDestroy() {
        isRunning = false; animation?.cancel(); collapse(); dock?.let { runCatching { windows.removeView(it) } }; dock = null
        if (receiverRegistered) unregisterReceiver(screenReceiver)
        getSystemService(AppOpsManager::class.java).stopWatchingMode(overlayPermissionListener)
        main.removeCallbacksAndMessages(null); scope.cancel(); registry.handleLifecycleEvent(Lifecycle.Event.ON_DESTROY)
        stopForeground(STOP_FOREGROUND_REMOVE); super.onDestroy()
    }

    companion object {
        private const val CHANNEL = "workflow_overlay"
        private const val NOTIFICATION_ID = 7410
        private const val ACTION_STOP = "top.flysoftbeta.workflow.overlay.STOP"
        private const val ACTION_RESET = "top.flysoftbeta.workflow.overlay.RESET"
        private val positionIo = Dispatchers.IO.limitedParallelism(1)
        private val positionWrites = Mutex()
        private val runningState = MutableStateFlow(false)
        val running: StateFlow<Boolean> = runningState.asStateFlow()
        private val failureState = MutableStateFlow<String?>(null)
        /** Failures that cannot be displayed by an overlay remain visible in its settings row. */
        val failure: StateFlow<String?> = failureState.asStateFlow()
        @Volatile var isRunning = false
            private set(value) { field = value; runningState.value = value }
        fun start(context: Context): Result<Unit> = runCatching {
            failureState.value = null
            WorkspaceConnectionManager.get(context).requireSession()
            check(android.provider.Settings.canDrawOverlays(context)) { "请先允许悬浮窗权限" }
            context.startForegroundService(Intent(context, WorkflowOverlayService::class.java)); Unit
        }.onFailure { failureState.value = it.message ?: "悬浮窗未启动" }
        fun stop(context: Context) { context.stopService(Intent(context, WorkflowOverlayService::class.java)) }
        /** Called from a foreground Activity after the store is ready; does not request permissions. */
        fun restoreIfEnabled(context: Context) {
            val session = WorkspaceConnectionManager.get(context).sessionOrNull() ?: return
            if (!isRunning && session.store.state.value.config.overlay.enabled && android.provider.Settings.canDrawOverlays(context)) start(context)
        }
        fun resetPosition(context: Context) {
            if (isRunning) context.startService(Intent(context, WorkflowOverlayService::class.java).setAction(ACTION_RESET))
            else {
                val documents = runCatching { WorkspaceDocuments(WorkspaceConnectionManager.get(context).requireSession().rpc) }
                    .getOrElse { failureState.value = it.message; return }
                AppGraph.processScope.launch(positionIo) {
                    runCatching { writePosition(documents, true, .42f) }.onFailure { failureState.value = it.message ?: "无法保存悬浮窗位置" }
                }
            }
        }
        private suspend fun writePosition(documents: WorkspaceDocuments, right: Boolean, fraction: Float) = positionWrites.withLock {
            val previous = documents.read("overlay", "position")
            val text = JSONObject().put("right", right).put("verticalFraction", OverlayPosition.safeFraction(fraction).toDouble()).toString()
            documents.write("overlay", "position", text, previous.revision)
        }
    }
}

internal object OverlayPosition {
    fun safeFraction(value: Float): Float = if (value.isFinite()) value.coerceIn(0f, 1f) else .42f
}
