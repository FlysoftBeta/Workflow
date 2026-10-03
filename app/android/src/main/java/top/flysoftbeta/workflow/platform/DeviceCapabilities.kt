package top.flysoftbeta.workflow.platform

import android.Manifest
import android.app.Activity
import android.app.NotificationManager
import android.app.admin.DeviceAdminReceiver
import android.app.admin.DevicePolicyManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.provider.Settings
import org.json.JSONObject
import kotlinx.coroutines.runBlocking
import top.flysoftbeta.workflow.client.sync.WorkspaceDocuments
import top.flysoftbeta.workflow.platform.connection.WorkspaceConnectionManager
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

class WorkflowDeviceAdminReceiver : DeviceAdminReceiver()

enum class RootAccess { UNCHECKED, AVAILABLE, UNAVAILABLE }

data class CapabilitySnapshot(
    val overlayGranted: Boolean,
    val writeSettingsGranted: Boolean,
    val deviceAdminActive: Boolean,
    val notificationsGranted: Boolean,
    val overlayRunning: Boolean,
    val rootAccess: RootAccess,
    val rootMessage: String,
    val grayscaleEnabled: Boolean,
    val brightness: Int,
)

/** Permission reads never run su or request a permission. Root is probed only by an explicit action. */
class DeviceCapabilities(context: Context) {
    private val permissionActivity = context as? Activity
    private val context = context.applicationContext
    private val policy get() = context.getSystemService(DevicePolicyManager::class.java)
    private val admin get() = ComponentName(context, WorkflowDeviceAdminReceiver::class.java)

    fun snapshot(): CapabilitySnapshot = CapabilitySnapshot(
        overlayGranted = Settings.canDrawOverlays(context),
        writeSettingsGranted = Settings.System.canWrite(context),
        deviceAdminActive = policy.isAdminActive(admin),
        notificationsGranted = (Build.VERSION.SDK_INT < 33 || context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED)
            && context.getSystemService(NotificationManager::class.java).areNotificationsEnabled(),
        overlayRunning = WorkflowOverlayService.isRunning && Settings.canDrawOverlays(context),
        rootAccess = RootCommands.access,
        rootMessage = RootCommands.message,
        grayscaleEnabled = Settings.Secure.getInt(context.contentResolver, "accessibility_display_daltonizer_enabled", 0) == 1
            && Settings.Secure.getInt(context.contentResolver, "accessibility_display_daltonizer", -1) == 0,
        brightness = Settings.System.getInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS, 128).coerceIn(1, 255),
    )

    fun requestOverlayPermission(): Result<Unit> = launch(Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, packageUri()))
    fun requestWriteSettings(): Result<Unit> = launch(Intent(Settings.ACTION_MANAGE_WRITE_SETTINGS, packageUri()))
    fun requestDeviceAdmin(): Result<Unit> = runCatching {
        // Android 9's DeviceAdminAdd rejects FLAG_ACTIVITY_NEW_TASK. Keep this grant in the caller's Activity task.
        checkNotNull(permissionActivity) { "请在设置的权限页面启用锁屏权限" }.startActivity(
            Intent(DevicePolicyManager.ACTION_ADD_DEVICE_ADMIN)
                .putExtra(DevicePolicyManager.EXTRA_DEVICE_ADMIN, admin)
                .putExtra(DevicePolicyManager.EXTRA_ADD_EXPLANATION, "允许 Workflow 在你点击锁屏时立即锁定屏幕。此权限不会读取你的文件。")
        )
    }
    fun openNotificationSettings(): Result<Unit> = launch(Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
        .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName))
    fun requestNotificationPermission(activity: Activity, requestCode: Int = 7401) {
        if (Build.VERSION.SDK_INT >= 33) activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), requestCode)
    }
    fun revokeDeviceAdmin(): Result<Unit> = runCatching { policy.removeActiveAdmin(admin) }

    /** Values are applied only after a user moves the control; automatic brightness is then disabled. */
    fun setBrightness(value: Int): Result<Unit> = runCatching {
        check(Settings.System.canWrite(context)) { "请先在设置的权限页面允许修改系统设置" }
        check(Settings.System.putInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS_MODE, Settings.System.SCREEN_BRIGHTNESS_MODE_MANUAL)) { "系统拒绝关闭自动亮度" }
        val target = value.coerceIn(1, 255)
        check(Settings.System.putInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS, target)) { "系统未接受亮度调整" }
        check(Settings.System.getInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS, -1) == target &&
            Settings.System.getInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS_MODE, -1) == Settings.System.SCREEN_BRIGHTNESS_MODE_MANUAL) { "亮度调整未生效" }
    }

    /** Callback always runs on the main thread; su timeout/denial is reported instead of assumed success. */
    fun probeRoot(callback: (CapabilitySnapshot) -> Unit) = RootCommands.submit({
        RootCommands.run("id -u").also { result ->
            RootCommands.access = if (result.success && result.output.trim() == "0") RootAccess.AVAILABLE else RootAccess.UNAVAILABLE
            RootCommands.message = if (RootCommands.access == RootAccess.AVAILABLE) "Root 已授权" else result.explanation("未获得 Root 权限")
        }
    }) { callback(snapshot()) }

    fun setGrayscale(enabled: Boolean, callback: (Result<CapabilitySnapshot>) -> Unit) = RootCommands.submit({
        runCatching {
            // RootCommands serializes this work off the main thread. Restoration is committed to the
            // Engine before touching the system, with no local workspace-state fallback.
            val documents = WorkspaceDocuments(WorkspaceConnectionManager.get(context).requireSession().rpc)
            val document = runBlocking { documents.read("device-controls", "grayscale-restore") }
            val beforeEnabled = Settings.Secure.getInt(context.contentResolver, "accessibility_display_daltonizer_enabled", 0)
            val beforeMode = Settings.Secure.getInt(context.contentResolver, "accessibility_display_daltonizer", -1)
            val previous = document.text?.let(::JSONObject)
            val targetEnabled = if (enabled) 1 else previous?.optInt("colorEnabled", 0) ?: 0
            val targetMode = if (enabled) 0 else previous?.optInt("colorMode", -1) ?: -1
            // Keep restoration data before changing the device, so an interrupted service can still restore it.
            if (enabled && !(beforeEnabled == 1 && beforeMode == 0)) {
                val restore = JSONObject().put("colorEnabled", beforeEnabled).put("colorMode", beforeMode).toString()
                runBlocking { documents.write("device-controls", "grayscale-restore", restore, document.revision) }
            }
            val command = "id -u; test \"\$(id -u)\" = 0 && settings put secure accessibility_display_daltonizer $targetMode && settings put secure accessibility_display_daltonizer_enabled $targetEnabled && settings get secure accessibility_display_daltonizer && settings get secure accessibility_display_daltonizer_enabled"
            val result = RootCommands.run(command)
            val lines = result.output.lineSequence().map(String::trim).filter(String::isNotEmpty).toList()
            RootCommands.access = if (lines.firstOrNull() == "0") RootAccess.AVAILABLE else RootAccess.UNAVAILABLE
            RootCommands.message = if (RootCommands.access == RootAccess.AVAILABLE) "Root 已授权" else result.explanation("未获得 Root 权限")
            check(result.success && lines.firstOrNull() == "0" && lines.takeLast(2) == listOf(targetMode.toString(), targetEnabled.toString())) { result.explanation("系统未接受黑白模式设置") }
            if (!enabled) runBlocking { documents.write("device-controls", "grayscale-restore", "{}", document.revision) }
            snapshot()
        }
    }, callback)

    fun lockScreen(callback: (Result<Unit>) -> Unit) {
        if (policy.isAdminActive(admin)) {
            val sent = runCatching { policy.lockNow() }
            if (sent.isFailure) { callback(sent); return }
            RootCommands.submit({ runCatching { awaitScreenOff() } }, callback)
            return
        }
        RootCommands.submit({
            runCatching {
                val result = RootCommands.run("id -u; test \"\$(id -u)\" = 0 && input keyevent 223")
                val rootGranted = result.output.lineSequence().firstOrNull()?.trim() == "0"
                RootCommands.access = if (rootGranted) RootAccess.AVAILABLE else RootAccess.UNAVAILABLE
                RootCommands.message = if (rootGranted) "Root 已授权" else result.explanation("未获得 Root 权限")
                check(result.success && rootGranted) { result.explanation("请在设置的权限页面启用锁屏授权，或授予 Root 权限") }
                // Power state changes asynchronously after the input event is dispatched.
                awaitScreenOff()
            }
        }, callback)
    }

    private fun awaitScreenOff() {
        var attempts = 0
        while (context.getSystemService(PowerManager::class.java).isInteractive && attempts++ < 15) Thread.sleep(80)
        check(!context.getSystemService(PowerManager::class.java).isInteractive) { "锁屏命令已发送，但屏幕未进入休眠" }
    }

    private fun packageUri() = Uri.parse("package:${context.packageName}")
    private fun launch(intent: Intent): Result<Unit> = runCatching { context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
}

private data class RootResult(val exitCode: Int, val output: String, val error: String? = null) {
    val success get() = exitCode == 0 && error == null
    fun explanation(fallback: String) = error ?: output.trim().takeIf(String::isNotEmpty)?.take(180) ?: fallback
}

private object RootCommands {
    @Volatile var access = RootAccess.UNCHECKED
    @Volatile var message = "尚未检查；仅黑白模式和 Root 锁屏需要"
    private val worker = Executors.newSingleThreadExecutor { runnable -> Thread(runnable, "workflow-device-controls").apply { isDaemon = true } }
    private val main = Handler(Looper.getMainLooper())

    fun <T> submit(action: () -> T, callback: (T) -> Unit) {
        worker.execute { val result = action(); main.post { callback(result) } }
    }

    fun run(command: String): RootResult = try {
        val process = ProcessBuilder("su", "-c", command).redirectErrorStream(true).start()
        val buffer = StringBuffer()
        val reader = Thread({
            runCatching {
                process.inputStream.bufferedReader().use { stream ->
                    val chars = CharArray(1024)
                    while (true) {
                        val count = stream.read(chars)
                        if (count < 0) break
                        if (buffer.length < 16_384) buffer.append(chars, 0, count.coerceAtMost(16_384 - buffer.length))
                    }
                }
            }
        }, "workflow-su-output").apply { isDaemon = true; start() }
        if (!process.waitFor(15, TimeUnit.SECONDS)) {
            process.destroyForcibly()
            reader.join(500)
            RootResult(-1, buffer.toString(), "Root 授权超时，请在权限管理器中允许后重试")
        } else {
            reader.join(500)
            RootResult(process.exitValue(), buffer.toString())
        }
    } catch (error: Exception) {
        RootResult(-1, "", if (error is java.io.IOException) "设备没有可用的 su，或 Root 权限被拒绝" else error.message ?: "Root 命令执行失败")
    }
}
