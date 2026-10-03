package top.flysoftbeta.workflow.ui.design

import androidx.annotation.DrawableRes
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Text
import androidx.compose.material3.TooltipAnchorPosition
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.rememberTooltipState
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** A bundled Material Symbol (see [top.flysoftbeta.workflow.ui.design.icons.Sym]) tinted with the content color. */
@Composable
fun SymbolIcon(
    @DrawableRes icon: Int,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    size: Dp = WorkflowTheme.dimens.icon,
    tint: Color = LocalContentColor.current,
) {
    Icon(painterResource(icon), contentDescription, modifier.size(size), tint = tint)
}

/**
 * Compact icon button (docs/ux/README.md §1.1 touch policy).
 *
 * - Layout cell = touch size (compact 36, standard 48); the ripple/checked indicator is the visual size
 *   (32 / 40) centred in it, so adjacent buttons in a tool row never overlap. Inside a bar that is
 *   lower than the touch size the parent's height constraint wins (standard: 48 wide × 44 high).
 * - Long press shows a PlainTooltip with [contentDescription] unless [onLongClick] is given.
 * - [badge] draws a 6dp dot (e.g. tertiary for pending requests on ◨).
 * - [checked] (non-null) makes it a toggle: secondaryContainer indicator + stateDescription.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun WfIconButton(
    @DrawableRes icon: Int,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    checked: Boolean? = null,
    tint: Color = Color.Unspecified,
    badge: Color? = null,
    iconSize: Dp = WorkflowTheme.dimens.icon,
    onLongClick: (() -> Unit)? = null,
) {
    val dimens = WorkflowTheme.dimens
    val colors = WorkflowTheme.colors
    val interaction = remember { MutableInteractionSource() }
    val haptics = LocalHapticFeedback.current
    val contentColor = when {
        tint != Color.Unspecified -> tint
        checked == true -> colors.onSecondaryContainer
        else -> colors.onSurfaceVariant
    }
    val button = @Composable {
        Box(
            modifier
                .size(dimens.iconButtonTouch)
                .combinedClickable(
                    interactionSource = interaction,
                    indication = ripple(bounded = false, radius = dimens.iconButton / 2),
                    enabled = enabled,
                    role = Role.Button,
                    onLongClick = onLongClick?.let { { haptics.performHapticFeedback(HapticFeedbackType.LongPress); it() } },
                    onClick = onClick,
                )
                .semantics {
                    this.contentDescription = contentDescription
                    if (checked != null) stateDescription = if (checked) "开" else "关"
                },
            contentAlignment = Alignment.Center,
        ) {
            if (checked == true) {
                Box(Modifier.size(dimens.iconButton).background(colors.secondaryContainer, CircleShape))
            }
            Icon(
                painterResource(icon), null,
                Modifier.size(iconSize).alpha(if (enabled) 1f else 0.38f),
                tint = contentColor,
            )
            if (badge != null) {
                Box(
                    Modifier.align(Alignment.Center)
                        .offset(x = iconSize / 2, y = -iconSize / 2)
                        .size(6.dp)
                        .background(badge, CircleShape)
                )
            }
        }
    }
    if (onLongClick != null) {
        button()
    } else {
        TooltipBox(
            positionProvider = TooltipDefaults.rememberTooltipPositionProvider(TooltipAnchorPosition.Below),
            tooltip = { PlainTooltip { Text(contentDescription) } },
            state = rememberTooltipState(),
        ) { button() }
    }
}
