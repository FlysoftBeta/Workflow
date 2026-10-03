package top.flysoftbeta.workflow.platform.apps

import android.content.ActivityNotFoundException
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.core.graphics.drawable.toBitmap
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import top.flysoftbeta.workflow.core.config.AppRef

/** A launchable activity of another app, with its label and icon loaded off the main thread. */
@Immutable
data class InstalledApp(
    val ref: AppRef,
    /** Activity label (what launchers show). */
    val label: String,
    /** Application label, used only to tell two equal labels apart. */
    val appLabel: String,
    val icon: ImageBitmap?,
)

/**
 * Launchable apps from the PackageManager (docs/ui.md §1.3: loaded and cached on a background thread;
 * the main thread never calls `loadLabel` / `loadIcon`). Process-wide.
 */
class InstalledApps private constructor(private val context: Context) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val mutex = Mutex()
    private val iconPx = (56 * context.resources.displayMetrics.density).toInt().coerceAtLeast(48)

    private val mutableApps = MutableStateFlow<List<InstalledApp>>(emptyList())
    /** All launchable activities of other apps, sorted by label (filled by [refresh]). */
    val apps: StateFlow<List<InstalledApp>> = mutableApps.asStateFlow()

    private val mutableKnown = MutableStateFlow<Map<String, InstalledApp>>(emptyMap())
    /** Info by [AppRef.id] for the refs requested through [ensure] or found by [refresh]. */
    val known: StateFlow<Map<String, InstalledApp>> = mutableKnown.asStateFlow()

    private val missing = java.util.concurrent.ConcurrentHashMap.newKeySet<String>()

    /** Reloads the list of launchable activities (add-apps sheet, package changes). */
    fun refresh() {
        scope.launch {
            mutex.withLock {
                val pm = context.packageManager
                val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
                val activities = runCatching { pm.queryIntentActivities(intent, 0) }.getOrDefault(emptyList())
                    .filter { it.activityInfo.packageName != context.packageName }
                val perPackage = activities.groupingBy { it.activityInfo.packageName }.eachCount()
                missing.clear()
                val list = activities.mapNotNull { resolve ->
                    val info = resolve.activityInfo
                    val ref = runCatching {
                        if ((perPackage[info.packageName] ?: 0) > 1) AppRef(info.packageName, info.name) else AppRef(info.packageName)
                    }.getOrNull() ?: return@mapNotNull null
                    InstalledApp(
                        ref = ref,
                        label = runCatching { resolve.loadLabel(pm).toString() }.getOrDefault(info.packageName),
                        appLabel = runCatching { info.applicationInfo.loadLabel(pm).toString() }.getOrDefault(info.packageName),
                        icon = runCatching { resolve.loadIcon(pm).toBitmap(iconPx, iconPx).asImageBitmap() }.getOrNull(),
                    )
                }.sortedBy { it.label.lowercase() }
                mutableApps.value = list
                mutableKnown.value = mutableKnown.value + list.associateBy { it.ref.id }
            }
        }
    }

    /** Loads info for launcher entries not known yet (cached; uninstalled apps are remembered as missing). */
    fun ensure(refs: List<AppRef>) {
        val wanted = refs.filter { it.id !in mutableKnown.value && it.id !in missing }
        if (wanted.isEmpty()) return
        scope.launch {
            mutex.withLock {
                val pm = context.packageManager
                val found = wanted.mapNotNull { ref ->
                    val component = component(ref) ?: run { missing += ref.id; return@mapNotNull null }
                    runCatching {
                        val info = pm.getActivityInfo(component, 0)
                        InstalledApp(
                            ref = ref,
                            label = info.loadLabel(pm).toString(),
                            appLabel = info.applicationInfo.loadLabel(pm).toString(),
                            icon = runCatching { info.loadIcon(pm).toBitmap(iconPx, iconPx).asImageBitmap() }.getOrNull(),
                        )
                    }.getOrElse { missing += ref.id; null }
                }
                if (found.isNotEmpty()) mutableKnown.value = mutableKnown.value + found.associateBy { it.ref.id }
            }
        }
    }

    /** The launch component of [ref]: its explicit activity, else the package's launcher activity. */
    private fun component(ref: AppRef): ComponentName? {
        val activity = ref.activity
        if (activity != null) return ComponentName(ref.packageName, if (activity.startsWith(".")) ref.packageName + activity else activity)
        return context.packageManager.getLaunchIntentForPackage(ref.packageName)?.component
    }

    /** Starts [ref] as its own task. Returns false when it is not installed any more. */
    fun launch(ref: AppRef): Boolean {
        val component = runCatching { component(ref) }.getOrNull() ?: return false
        val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER).setComponent(component)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED)
        return try { context.startActivity(intent); true } catch (_: ActivityNotFoundException) { false } catch (_: SecurityException) { false }
    }

    /** "应用信息": the system app details page. */
    fun openInstalledApp(ref: AppRef) {
        val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:${ref.packageName}"))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        runCatching { context.startActivity(intent) }
    }

    fun isInstalled(ref: AppRef): Boolean = ref.id !in missing

    companion object {
        @Volatile private var instance: InstalledApps? = null

        fun get(context: Context): InstalledApps = instance ?: synchronized(this) {
            instance ?: InstalledApps(context.applicationContext).also { instance = it }
        }
    }
}
