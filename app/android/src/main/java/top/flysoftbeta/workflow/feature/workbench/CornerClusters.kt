package top.flysoftbeta.workflow.feature.workbench

import androidx.compose.foundation.layout.RowScope
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import top.flysoftbeta.workflow.app.sessionLabel
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.PanelTarget
import top.flysoftbeta.workflow.core.layout.Region
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.ui.design.SessionChip
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.dnd.AreaStyle
import top.flysoftbeta.workflow.ui.design.dnd.DropTargetKind
import top.flysoftbeta.workflow.ui.design.dnd.dropTarget
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import java.time.ZoneId

internal val LocalCompactWorkbenchHeader = staticCompositionLocalOf { false }

/**
 * Top-left corner cluster `[⌂][◧][session chip]` (docs/ux/README.md §2.1). [sideRegion] is the region ◧
 * toggles (explorer in Files, conversation list in Chat); null hides ◧ (Solo).
 */
@Composable
internal fun RowScope.LeftCluster(env: WorkbenchEnv, session: Session, sideRegion: Region?, sideShown: Boolean) {
    val shell = env.shell
    WfIconButton(Sym.Home, "主页", { shell.goHome() })
    if (sideRegion != null) {
        WfIconButton(
            if (sideShown) Sym.LeftPanelClose else Sym.LeftPanelOpen,
            if (sideRegion == Region.EXPLORER) "文件浏览器" else "对话列表",
            { env.runtime.toggleRegion(sideRegion) },
            modifier = Modifier.hoverOpens(env, sideRegion, sideShown),
        )
    }
    if (LocalCompactWorkbenchHeader.current) {
        WfIconButton(Sym.History, "所有会话", { shell.sessionsOpen = true })
    } else {
        SessionChip(
            label = sessionLabel(session, ZoneId.systemDefault()),
            onClick = { shell.sessionsOpen = true },
            onLongClick = { shell.renaming = session.id },
        )
    }
}

/**
 * Top-right corner cluster `│[◨][⚙]` (docs/ux/README.md §2.1). ◨ toggles the aux region (conversation in
 * Files, file side in Chat) and carries a tertiary dot while a request waits; ⚙ opens Settings in this
 * session (focused when already open).
 */
@Composable
internal fun RowScope.RightCluster(env: WorkbenchEnv, auxRegion: Region?, auxShown: Boolean, attention: Boolean, settings: Boolean = true) {
    if (auxRegion != null) {
        WfIconButton(
            if (auxShown) Sym.RightPanelClose else Sym.RightPanelOpen,
            if (auxRegion == Region.AUX) "对话面板" else "文件侧栏",
            { env.runtime.toggleRegion(auxRegion) },
            modifier = Modifier.hoverOpens(env, auxRegion, auxShown),
            badge = if (attention) WorkflowTheme.colors.tertiary else null,
        )
    }
    if (settings) WfIconButton(Sym.Settings, "设置", { env.layout(LayoutOp.Open(PanelTarget.Settings)) })
}

/** While dragging, resting 600ms on a collapsed region's toggle opens it (docs/ux/README.md §4.10). */
private fun Modifier.hoverOpens(env: WorkbenchEnv, region: Region, shown: Boolean): Modifier =
    if (shown) this else dropTarget(
        env.dnd, key = "toggle:${region.name}",
        kind = DropTargetKind.Area(AreaStyle.Self),
        priority = 2,
        onHoverActivate = { env.runtime.showRegion(region) },
        onDrop = { _, _ -> env.runtime.showRegion(region) },
    )
