package top.flysoftbeta.workflow.app

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import top.flysoftbeta.workflow.core.config.Appearance
import top.flysoftbeta.workflow.feature.launcher.LauncherScreen
import top.flysoftbeta.workflow.feature.sessions.SessionDialogs
import top.flysoftbeta.workflow.feature.sessions.SessionsSheet
import top.flysoftbeta.workflow.feature.workbench.WorkbenchRuntime
import top.flysoftbeta.workflow.feature.workbench.WorkbenchScreen
import top.flysoftbeta.workflow.ui.design.dnd.DragDropState
import top.flysoftbeta.workflow.ui.design.WfSnackbarHost
import top.flysoftbeta.workflow.ui.design.dnd.DragDropHost
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.UiDensity
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowSystemBarsEffect
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.ui.design.theme.frame
import top.flysoftbeta.workflow.core.config.Density as ConfigDensity
import top.flysoftbeta.workflow.core.config.ThemeMode as ConfigTheme

/**
 * Root of the single activity: theme and density from config.json, edge-to-edge insets, one
 * drag-and-drop host for the window, the current space (Launcher or Workbench), the session view and
 * the shell's dialogs and snackbars on top.
 */
@Composable
fun WorkflowApp(model: ShellViewModel) = WorkflowApp(model.shell, model.workbench, model.dnd)

/** The root composition; tests host it with an in-memory store. */
@Composable
fun WorkflowApp(shell: Shell, workbench: WorkbenchRuntime, dnd: DragDropState) {
    val appearance by shell.appearance.collectAsState()
    WorkflowDesignTheme(themeMode = appearance.themeMode(), density = appearance.uiDensity()) {
        WorkflowSystemBarsEffect()
        val base = LocalDensity.current
        CompositionLocalProvider(LocalDensity provides Density(base.density, base.fontScale * appearance.fontScale.toFloat())) {
            Box(
                Modifier
                    .fillMaxSize()
                    .background(WorkflowTheme.colors.frame)
                    .windowInsetsPadding(WindowInsets.safeDrawing),
            ) {
                DragDropHost(dnd, Modifier.fillMaxSize()) {
                    when (shell.space) {
                        Space.LAUNCHER -> LauncherScreen(shell)
                        Space.WORKBENCH -> WorkbenchScreen(shell, workbench, dnd)
                    }
                    SessionsSheet(shell)
                }
                SessionDialogs(shell)
                shell.decision?.let { pending ->
                    DecisionDialog(
                        title = pending.request.title,
                        message = pending.request.message,
                        items = pending.request.items,
                        options = pending.request.options,
                        cancelLabel = pending.request.cancelLabel,
                        onResult = { shell.answer(it) },
                    )
                }
                WfSnackbarHost(shell.snackbar, Modifier.fillMaxSize())
            }
        }
    }
}

private fun Appearance.themeMode(): ThemeMode = when (theme) {
    ConfigTheme.SYSTEM -> ThemeMode.System
    ConfigTheme.LIGHT -> ThemeMode.Light
    ConfigTheme.DARK -> ThemeMode.Dark
}

private fun Appearance.uiDensity(): UiDensity = when (density) {
    ConfigDensity.COMPACT -> UiDensity.Compact
    ConfigDensity.STANDARD -> UiDensity.Standard
}
