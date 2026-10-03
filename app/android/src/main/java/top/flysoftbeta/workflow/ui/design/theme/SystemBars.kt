package top.flysoftbeta.workflow.ui.design.theme

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext

private tailrec fun Context.findActivity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.findActivity()
    else -> null
}

/**
 * Edge-to-edge with frame-colored system bars (docs/ux/README.md §2.1): status bar and 3-button navigation bar
 * use `surfaceContainer`, icons follow the theme, the status bar stays visible. Call inside
 * [WorkflowDesignTheme]; the root layout must still pad for `WindowInsets.safeDrawing`.
 */
@Composable
fun WorkflowSystemBarsEffect() {
    val activity = LocalContext.current.findActivity() as? ComponentActivity ?: return
    val frame = WorkflowTheme.colors.frame.toArgb()
    val dark = WorkflowTheme.isDark
    // The activity handles rotation / density itself (no recreation); API 28 resets the bar colors then.
    val configuration = androidx.compose.ui.platform.LocalConfiguration.current
    DisposableEffect(activity, frame, dark, configuration.orientation, configuration.densityDpi, configuration.screenWidthDp) {
        val style = if (dark) SystemBarStyle.dark(frame) else SystemBarStyle.light(frame, frame)
        activity.enableEdgeToEdge(statusBarStyle = style, navigationBarStyle = style)
        onDispose { }
    }
}
