package top.flysoftbeta.workflow.ui.design.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** The two density levels of docs/ui.md §1.1. Switched at runtime from config ("设置 → 外观"). */
enum class UiDensity { Compact, Standard }

/**
 * Dimension tokens (docs/ui.md §1.1). Components read these instead of hard-coding sizes.
 * Heights are minimums: rows use `heightIn(min = …)` so font scaling up to 1.3 never clips text (§7).
 */
@Immutable
data class WorkflowDimens(
    val density: UiDensity,
    /** Tab row, region header, find bar. */
    val bar: Dp,
    /** Icon button visual cell (the 36 in "32/36" is [iconButtonTouch]). */
    val iconButton: Dp,
    /** Icon button touch cell; also provided as LocalMinimumInteractiveComponentSize. */
    val iconButtonTouch: Dp,
    /** Default icon size. */
    val icon: Dp,
    /** Icons in menus and the file tree. */
    val iconSmall: Dp,
    /** File tree and conversation list rows. */
    val treeRow: Dp,
    /** Single-line list row (session view, add app). */
    val listRow: Dp,
    /** Two-line list row. */
    val listRowTwoLine: Dp,
    /** Settings rows are fixed at 48 in both densities. */
    val settingsRow: Dp,
    val menuItem: Dp,
    val extraKeysRow: Dp,
    val extraKey: Dp,
    /** Seam between regions and to window edges. */
    val gap: Dp,
    /** Horizontal content padding. */
    val padH: Dp,
    /** Tree indent per level. */
    val indent: Dp,
) {
    // Density-independent sizes from §2–§4.
    val tabMinWidth: Dp get() = 72.dp
    val tabMaxWidth: Dp get() = 220.dp
    val tabIcon: Dp get() = 16.dp
    val tabTrailingSlot: Dp get() = 24.dp
    val dirtyDot: Dp get() = 8.dp
    /** Stand-alone controls (send, switch, slider). */
    val controlMin: Dp get() = 40.dp
    /** Launcher and floating overlay targets. */
    val launcherTouchMin: Dp get() = 48.dp
    /** Tool row surfaced actions collapse into More below this remaining tab width (§2.2). */
    val surfacedActionsMinTabSpace: Dp get() = 120.dp
    /** Divider seam hit area and pill handle (§3.2). */
    val dividerTouch: Dp get() = 20.dp
    val dividerHandleLength: Dp get() = 32.dp
    val dividerHandleThickness: Dp get() = 4.dp
    /** Group header of time-grouped lists. */
    val groupHeader: Dp get() = 28.dp
    val activityRow: Dp get() = 28.dp

    companion object {
        val Compact = WorkflowDimens(
            density = UiDensity.Compact, bar = 32.dp, iconButton = 28.dp, iconButtonTouch = 32.dp,
            icon = 20.dp, iconSmall = 18.dp, treeRow = 32.dp, listRow = 36.dp, listRowTwoLine = 48.dp,
            settingsRow = 48.dp, menuItem = 36.dp, extraKeysRow = 40.dp, extraKey = 34.dp,
            gap = 4.dp, padH = 12.dp, indent = 12.dp,
        )
        val Standard = WorkflowDimens(
            density = UiDensity.Standard, bar = 44.dp, iconButton = 40.dp, iconButtonTouch = 48.dp,
            icon = 24.dp, iconSmall = 24.dp, treeRow = 40.dp, listRow = 48.dp, listRowTwoLine = 64.dp,
            settingsRow = 48.dp, menuItem = 44.dp, extraKeysRow = 48.dp, extraKey = 40.dp,
            gap = 6.dp, padH = 16.dp, indent = 16.dp,
        )

        fun of(density: UiDensity): WorkflowDimens = when (density) {
            UiDensity.Compact -> Compact
            UiDensity.Standard -> Standard
        }
    }
}

/** Corner radius tokens (docs/ui.md §1.2). */
object WorkflowRadii {
    val xs: Dp = 4.dp
    val sm: Dp = 8.dp
    val md: Dp = 12.dp
    val lg: Dp = 20.dp
    val xl: Dp = 24.dp
}

/** Shapes for the radius tokens; `full` is a pill/circle. */
@Immutable
object WorkflowShapes {
    val xs = RoundedCornerShape(WorkflowRadii.xs)
    val sm = RoundedCornerShape(WorkflowRadii.sm)
    val md = RoundedCornerShape(WorkflowRadii.md)
    val lg = RoundedCornerShape(WorkflowRadii.lg)
    val xl = RoundedCornerShape(WorkflowRadii.xl)
    val full = RoundedCornerShape(percent = 50)
    /** Tab: rounded top corners only, joins the content below. */
    val tab = RoundedCornerShape(topStart = WorkflowRadii.sm, topEnd = WorkflowRadii.sm)
}

val LocalWorkflowDimens = staticCompositionLocalOf { WorkflowDimens.Compact }
