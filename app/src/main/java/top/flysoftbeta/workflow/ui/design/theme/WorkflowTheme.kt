package top.flysoftbeta.workflow.ui.design.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LocalMinimumInteractiveComponentSize
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.Shapes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode

/** Theme preference (docs/ui.md §1.3): follow system, light or dark. Editor and terminal follow it too. */
enum class ThemeMode { System, Light, Dark }

val LocalIsDarkTheme = staticCompositionLocalOf { false }

private val WorkflowM3Shapes = Shapes(
    extraSmall = WorkflowShapes.xs,
    small = WorkflowShapes.sm,
    medium = WorkflowShapes.md,
    large = WorkflowShapes.lg,
    extraLarge = WorkflowShapes.xl,
)

/**
 * Root theme of the rewritten UI: Material 3 Expressive with the generated seed scheme (no dynamic
 * color on API 28), the ui.md type scale, radius tokens, `MotionScheme.standard()` (fast, no bounce)
 * and the density tokens. [density] and [themeMode] may change at runtime (config.json).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun WorkflowDesignTheme(
    themeMode: ThemeMode = ThemeMode.System,
    density: UiDensity = UiDensity.Compact,
    content: @Composable () -> Unit,
) {
    val dark = when (themeMode) {
        ThemeMode.System -> isSystemInDarkTheme()
        ThemeMode.Light -> false
        ThemeMode.Dark -> true
    }
    val context = LocalContext.current
    val inspection = LocalInspectionMode.current
    val styles = remember(context, inspection) {
        workflowTextStyles(monoFontFamily(if (inspection) null else context.assets))
    }
    val typography = remember(styles) { workflowTypography(styles) }
    val dimens = WorkflowDimens.of(density)
    MaterialExpressiveTheme(
        colorScheme = if (dark) WorkflowDarkColorScheme else WorkflowLightColorScheme,
        motionScheme = MotionScheme.standard(),
        shapes = WorkflowM3Shapes,
        typography = typography,
    ) {
        CompositionLocalProvider(
            LocalWorkflowDimens provides dimens,
            LocalWorkflowTextStyles provides styles,
            LocalExtendedColors provides if (dark) GeneratedDarkExtendedColors else GeneratedLightExtendedColors,
            LocalMinimumInteractiveComponentSize provides dimens.iconButtonTouch,
            LocalIsDarkTheme provides dark,
            content = content,
        )
    }
}

/** Accessors for the design tokens inside [WorkflowDesignTheme]. */
object WorkflowTheme {
    val dimens: WorkflowDimens
        @Composable @ReadOnlyComposable get() = LocalWorkflowDimens.current
    val text: WorkflowTextStyles
        @Composable @ReadOnlyComposable get() = LocalWorkflowTextStyles.current
    val colors: ColorScheme
        @Composable @ReadOnlyComposable get() = MaterialTheme.colorScheme
    val extendedColors: ExtendedColors
        @Composable @ReadOnlyComposable get() = LocalExtendedColors.current
    val isDark: Boolean
        @Composable @ReadOnlyComposable get() = LocalIsDarkTheme.current
    val motion: MotionScheme
        @Composable @ReadOnlyComposable get() = MaterialTheme.motionScheme
}
