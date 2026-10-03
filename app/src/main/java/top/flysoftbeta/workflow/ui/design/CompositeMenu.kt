package top.flysoftbeta.workflow.ui.design

import android.content.res.Configuration
import androidx.annotation.DrawableRes
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenuGroup
import androidx.compose.material3.DropdownMenuPopup
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MenuAnchorPosition
import androidx.compose.material3.MenuDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.toggleableState
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** One entry of a [CompositeMenu]. Every entry carries an 18dp icon (docs/ui.md §2.2). */
@Immutable
sealed interface MenuEntry {
    val key: String
    val label: String
    val icon: Int?
    val enabled: Boolean

    /** Plain action. [destructive] entries use the error color and belong at the end of their group. */
    data class Action(
        override val key: String,
        override val label: String,
        @param:DrawableRes override val icon: Int?,
        override val enabled: Boolean = true,
        val destructive: Boolean = false,
        /** Shown right-aligned only while a hardware keyboard is attached, e.g. "Ctrl+S". */
        val shortcut: String? = null,
        val onClick: () -> Unit,
    ) : MenuEntry

    /** Check item ("☐自动换行"). The menu stays open after toggling. */
    data class Toggle(
        override val key: String,
        override val label: String,
        val checked: Boolean,
        override val enabled: Boolean = true,
        val onCheckedChange: (Boolean) -> Unit,
    ) : MenuEntry {
        override val icon: Int get() = if (checked) Sym.CheckBox else Sym.CheckBoxOutlineBlank
    }

    /** Opens a side submenu ("移动到 ▸"). */
    data class Submenu(
        override val key: String,
        override val label: String,
        @param:DrawableRes override val icon: Int?,
        val groups: List<MenuGroup>,
        override val enabled: Boolean = true,
    ) : MenuEntry
}

/** A group of entries; groups are separated visually (① resource ② panel/layout ③ session). */
@Immutable
data class MenuGroup(val key: String, val entries: List<MenuEntry>)

/** True while a hardware keyboard is attached (shortcut hints are shown only then). */
@Composable
fun hardwareKeyboardAttached(): Boolean {
    val configuration = LocalConfiguration.current
    return configuration.keyboard != Configuration.KEYBOARD_NOKEYS &&
        configuration.hardKeyboardHidden == Configuration.HARDKEYBOARDHIDDEN_NO
}

/** How groups are separated. [Segmented] uses the Expressive `DropdownMenuGroup` surfaces; [Dividers] is the fallback. */
enum class MenuGroupStyle { Segmented, Dividers }

/**
 * The composite More / long-press menu (docs/ui.md §2.2). Anchor it like `DropdownMenu`: place it in a
 * `Box` together with its anchor. Item height comes from the density tokens (`menuItem` 36/44), not from
 * M3's 48dp items. Empty groups are dropped. Submenus open to the side.
 */
@Composable
fun CompositeMenu(
    expanded: Boolean,
    onDismissRequest: () -> Unit,
    groups: List<MenuGroup>,
    modifier: Modifier = Modifier,
    anchorPosition: MenuAnchorPosition = MenuAnchorPosition.Below,
    offset: DpOffset = DpOffset(0.dp, 0.dp),
    style: MenuGroupStyle = MenuGroupStyle.Segmented,
    showShortcuts: Boolean = hardwareKeyboardAttached(),
) {
    DropdownMenuPopup(
        expanded = expanded,
        onDismissRequest = onDismissRequest,
        modifier = modifier,
        popupPositionProvider = MenuDefaults.rememberDropdownMenuPopupPositionProvider(anchorPosition, offset),
    ) {
        MenuGroups(groups, style, showShortcuts, onDismissRequest)
    }
}

@Composable
private fun MenuGroups(groups: List<MenuGroup>, style: MenuGroupStyle, showShortcuts: Boolean, dismiss: () -> Unit) {
    val visible = groups.filter { it.entries.isNotEmpty() }
    val container = WorkflowTheme.colors.surfaceContainerHigh
    val content: @Composable (MenuGroup) -> Unit = { group ->
        group.entries.forEach { entry -> MenuEntryRow(entry, showShortcuts, dismiss) }
    }
    Column(Modifier.widthIn(min = 180.dp, max = 320.dp).verticalScroll(rememberScrollState())) {
        when (style) {
            MenuGroupStyle.Segmented -> visible.forEachIndexed { index, group ->
                DropdownMenuGroup(
                    shapes = MenuDefaults.groupShape(index, visible.size),
                    containerColor = container,
                    contentPadding = PaddingValues(vertical = 4.dp),
                ) { content(group) }
                if (index != visible.lastIndex) Spacer(Modifier.height(MenuDefaults.GroupSpacing))
            }
            MenuGroupStyle.Dividers -> DropdownMenuGroup(
                shapes = MenuDefaults.groupShape(0, 1),
                containerColor = container,
                contentPadding = PaddingValues(vertical = 4.dp),
            ) {
                visible.forEachIndexed { index, group ->
                    content(group)
                    if (index != visible.lastIndex) HorizontalDivider(Modifier.padding(vertical = 4.dp))
                }
            }
        }
    }
}

@Composable
private fun MenuEntryRow(entry: MenuEntry, showShortcuts: Boolean, dismiss: () -> Unit) {
    val dimens = WorkflowTheme.dimens
    val colors = WorkflowTheme.colors
    val destructive = entry is MenuEntry.Action && entry.destructive
    val color = if (destructive) colors.error else colors.onSurface
    val iconColor = if (destructive) colors.error else colors.onSurfaceVariant
    var submenuOpen by remember { mutableStateOf(false) }
    Box {
        Row(
            Modifier
                .fillMaxWidth()
                .heightIn(min = dimens.menuItem)
                .clickable(enabled = entry.enabled) {
                    when (entry) {
                        is MenuEntry.Action -> { dismiss(); entry.onClick() }
                        is MenuEntry.Toggle -> entry.onCheckedChange(!entry.checked)
                        is MenuEntry.Submenu -> submenuOpen = true
                    }
                }
                .semantics {
                    if (entry is MenuEntry.Toggle) {
                        role = Role.Checkbox
                        toggleableState = ToggleableState(entry.checked)
                    }
                }
                .padding(horizontal = 12.dp)
                .alpha(if (entry.enabled) 1f else 0.38f),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val icon = entry.icon
            if (icon != null) {
                SymbolIcon(
                    icon, null, size = dimens.iconSmall,
                    tint = if (entry is MenuEntry.Toggle && entry.checked) colors.primary else iconColor,
                )
            } else {
                Spacer(Modifier.size(dimens.iconSmall))
            }
            Spacer(Modifier.width(12.dp))
            Text(
                entry.label, Modifier.weight(1f), style = WorkflowTheme.text.body, color = color,
                maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            when {
                entry is MenuEntry.Submenu -> SymbolIcon(Sym.ChevronRight, null, size = dimens.iconSmall, tint = iconColor)
                entry is MenuEntry.Action && showShortcuts && entry.shortcut != null -> Text(
                    entry.shortcut, Modifier.padding(start = 16.dp), style = WorkflowTheme.text.caption,
                    color = colors.onSurfaceVariant,
                )
            }
        }
        if (entry is MenuEntry.Submenu) {
            DropdownMenuPopup(
                expanded = submenuOpen,
                onDismissRequest = { submenuOpen = false },
                popupPositionProvider = MenuDefaults.rememberDropdownMenuPopupPositionProvider(MenuAnchorPosition.End),
            ) {
                MenuGroups(entry.groups, MenuGroupStyle.Segmented, showShortcuts) { submenuOpen = false; dismiss() }
            }
        }
    }
}

/** Convenience: an icon button that opens a [CompositeMenu] ("⋯"). */
@Composable
fun MoreMenuButton(
    groups: List<MenuGroup>,
    modifier: Modifier = Modifier,
    contentDescription: String = "更多",
    icon: Int = Sym.MoreHoriz,
    badge: Color? = null,
) {
    var open by remember { mutableStateOf(false) }
    Box(modifier) {
        WfIconButton(icon, contentDescription, onClick = { open = true }, badge = badge)
        CompositeMenu(expanded = open, onDismissRequest = { open = false }, groups = groups)
    }
}
