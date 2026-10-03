package top.flysoftbeta.workflow.ui.design.theme

import androidx.compose.material3.ColorScheme
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color

/** Roles outside the M3 scheme (docs/ux/README.md §1.3): success (diff +, good latency) and warning (full access, conflicts). */
@Immutable
data class ExtendedColors(
    val success: Color,
    val onSuccess: Color,
    val successContainer: Color,
    val onSuccessContainer: Color,
    val warning: Color,
    val onWarning: Color,
    val warningContainer: Color,
    val onWarningContainer: Color,
)

val LocalExtendedColors = staticCompositionLocalOf { GeneratedLightExtendedColors }

val WorkflowLightColorScheme: ColorScheme get() = GeneratedLightColorScheme
val WorkflowDarkColorScheme: ColorScheme get() = GeneratedDarkColorScheme

/*
 * Usage roles (docs/ux/README.md §1.3), spelled out so call sites stay consistent:
 *  frame (window background, status and navigation bars) = surfaceContainer
 *  region content = surface; menus, popovers, composer = surfaceContainerHigh
 *  code blocks = surfaceContainerHighest; selected row = secondaryContainer; focus/in progress = primary
 */
val ColorScheme.frame: Color get() = surfaceContainer
val ColorScheme.popover: Color get() = surfaceContainerHigh
val ColorScheme.codeBlock: Color get() = surfaceContainerHighest
val ColorScheme.selectedRow: Color get() = secondaryContainer
