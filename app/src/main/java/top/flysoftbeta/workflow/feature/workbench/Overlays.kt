package top.flysoftbeta.workflow.feature.workbench

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.core.layout.LayoutOp
import top.flysoftbeta.workflow.core.layout.StackId
import top.flysoftbeta.workflow.core.layout.Workbench
import top.flysoftbeta.workflow.ui.design.ItemListMenu
import top.flysoftbeta.workflow.ui.design.ListMenuItem
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.ui.design.theme.frame

/**
 * Regions that cannot dock (docs/ui.md §3.2): the side region as a drawer from the left, the aux region
 * sliding in from the right (min(400, W − 48), elevation 3); outside tap or Back closes them, their
 * state is kept. Below 600dp the drawer is modal with a stronger scrim and aux is full width.
 */
@Composable
internal fun RegionOverlays(
    env: WorkbenchEnv,
    sideOpen: Boolean,
    auxOpen: Boolean,
    narrow: Boolean,
    sideWidth: Dp,
    auxWidth: Dp,
    side: @Composable () -> Unit,
    aux: @Composable () -> Unit,
) {
    val presence = env.runtime.presence
    val colors = WorkflowTheme.colors
    val motion = WorkflowTheme.motion
    val gap = WorkflowTheme.dimens.gap
    val dragging = env.dnd.isDragging
    Box(Modifier.fillMaxSize()) {
        AnimatedVisibility(sideOpen || auxOpen, enter = fadeIn(motion.fastEffectsSpec()), exit = fadeOut(motion.fastEffectsSpec())) {
            Box(
                Modifier
                    .fillMaxSize()
                    .background(colors.scrim.copy(alpha = if (narrow) 0.32f else 0.16f))
                    .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                        presence.sideOverlay = false
                        presence.auxOverlay = false
                    },
            )
        }
        AnimatedVisibility(
            visible = sideOpen,
            enter = slideInHorizontally(motion.defaultSpatialSpec()) { -it },
            exit = slideOutHorizontally(motion.defaultSpatialSpec()) { -it },
            modifier = Modifier.align(Alignment.CenterStart),
        ) {
            Surface(Modifier.width(sideWidth).fillMaxHeight(), color = colors.frame, shadowElevation = 3.dp) {
                Box(Modifier.padding(gap)) { side() }
            }
        }
        AnimatedVisibility(
            visible = auxOpen,
            enter = slideInHorizontally(motion.defaultSpatialSpec()) { it },
            exit = slideOutHorizontally(motion.defaultSpatialSpec()) { it },
            modifier = Modifier.align(Alignment.CenterEnd),
        ) {
            Surface(Modifier.width(auxWidth).fillMaxHeight(), color = colors.frame, shadowElevation = 3.dp) {
                Box(Modifier.padding(gap)) { aux() }
            }
        }
    }
    BackHandler(enabled = (sideOpen || auxOpen) && !dragging) {
        if (auxOpen) presence.auxOverlay = false else presence.sideOverlay = false
    }
}

/** `[1/3 ▾]`: switches which stack a single-stack view shows (narrow Files, Chat side region). */
@Composable
internal fun StackSwitcher(env: WorkbenchEnv, wb: Workbench, current: StackId, candidates: List<StackId>, onSelect: ((StackId) -> Unit)? = null) {
    if (candidates.size < 2) return
    var open by remember { mutableStateOf(false) }
    val titleOf = { id: String -> wb.panels[id]?.let { env.controller(it).tab.title } }
    Box {
        Row(
            Modifier
                .height(WorkflowTheme.dimens.iconButtonTouch)
                .clip(WorkflowShapes.sm)
                .clickable { open = true }
                .semantics { role = Role.DropdownList; contentDescription = "切换 Stack" }
                .padding(horizontal = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("${candidates.indexOf(current) + 1}/${candidates.size}", style = WorkflowTheme.text.label, color = WorkflowTheme.colors.onSurfaceVariant)
            SymbolIcon(Sym.ArrowDropDown, null, size = 18.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
        }
        ItemListMenu(
            expanded = open,
            onDismissRequest = { open = false },
            items = candidates.map { id ->
                ListMenuItem(id, stackName(wb, id, titleOf), if (id == Workbench.BOTTOM) Sym.Terminal else Sym.Draft)
            },
            selectedKey = current,
            onSelect = { id ->
                open = false
                if (onSelect != null) onSelect(id) else env.layout(LayoutOp.FocusStack(id))
            },
        )
    }
}
