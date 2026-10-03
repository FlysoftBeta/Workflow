package top.flysoftbeta.workflow.ui.design

import androidx.annotation.DrawableRes
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenuGroup
import androidx.compose.material3.DropdownMenuPopup
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

@Immutable
data class ListMenuItem(
    val key: String,
    val title: String,
    @param:DrawableRes val icon: Int? = null,
    val caption: String? = null,
    val dot: Boolean = false,
)

/** Threshold above which [ItemListMenu] shows a search field (docs/ui.md §4.1). */
const val LIST_MENU_SEARCH_THRESHOLD = 8

/**
 * Anchored list popup (tab overflow `⌄n`, `[1/3 ▾]` stack switcher, conversation switcher).
 * The selected item uses the secondaryContainer pill; more than 8 items adds a filter field.
 */
@Composable
fun ItemListMenu(
    expanded: Boolean,
    onDismissRequest: () -> Unit,
    items: List<ListMenuItem>,
    selectedKey: String?,
    onSelect: (String) -> Unit,
    anchorPosition: MenuAnchorPosition = MenuAnchorPosition.Below,
) {
    var query by remember(expanded) { mutableStateOf("") }
    DropdownMenuPopup(
        expanded = expanded,
        onDismissRequest = onDismissRequest,
        modifier = Modifier.padding(MenuShadowPadding),
        popupPositionProvider = MenuDefaults.rememberDropdownMenuPopupPositionProvider(anchorPosition),
    ) {
        DropdownMenuGroup(
            shapes = MenuDefaults.groupShape(0, 1),
            containerColor = WorkflowTheme.colors.surfaceContainerHigh,
            contentPadding = PaddingValues(4.dp),
        ) {
            Column(Modifier.widthIn(min = 200.dp, max = 320.dp)) {
                if (items.size > LIST_MENU_SEARCH_THRESHOLD) {
                    SearchField(query, { query = it }, Modifier.fillMaxWidth().padding(4.dp))
                }
                val filtered = if (query.isBlank()) items else items.filter { it.title.contains(query.trim(), ignoreCase = true) }
                Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState())) {
                    filtered.forEach { item -> ListMenuRow(item, item.key == selectedKey) { onSelect(item.key) } }
                }
            }
        }
    }
}

@Composable
private fun ListMenuRow(item: ListMenuItem, selected: Boolean, onClick: () -> Unit) {
    val dimens = WorkflowTheme.dimens
    val colors = WorkflowTheme.colors
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = dimens.menuItem)
            .clip(WorkflowShapes.sm)
            .background(if (selected) colors.secondaryContainer else colors.surfaceContainerHigh)
            .clickable(onClick = onClick)
            .padding(horizontal = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (item.icon != null) {
            SymbolIcon(item.icon, null, size = dimens.iconSmall, tint = colors.onSurfaceVariant)
            Spacer(Modifier.width(10.dp))
        }
        Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
            Text(
                item.title, Modifier.weight(1f, fill = false),
                style = if (selected) WorkflowTheme.text.labelActive else WorkflowTheme.text.label,
                color = colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis,
            )
            if (item.caption != null) {
                Spacer(Modifier.width(6.dp))
                Text(item.caption, style = WorkflowTheme.text.caption, color = colors.onSurfaceVariant, maxLines = 1)
            }
        }
        if (item.dot) {
            Spacer(Modifier.width(8.dp))
            Box(Modifier.size(6.dp).background(colors.primary, CircleShape))
        }
    }
}

/** Compact search field (36dp by default, `sm` corners, surfaceContainerHighest). */
@Composable
fun SearchField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "搜索",
) {
    val colors = WorkflowTheme.colors
    val style = WorkflowTheme.text.body.copy(color = colors.onSurface)
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier,
        singleLine = true,
        textStyle = style,
        cursorBrush = SolidColor(colors.primary),
        decorationBox = { inner ->
            Row(
                Modifier
                    .heightIn(min = 32.dp)
                    .clip(WorkflowShapes.sm)
                    .background(colors.surfaceContainerHighest)
                    .padding(horizontal = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                SymbolIcon(Sym.Search, null, size = 18.dp, tint = colors.onSurfaceVariant)
                Spacer(Modifier.width(8.dp))
                Box(Modifier.weight(1f)) {
                    if (value.isEmpty()) Text(placeholder, style = style.copy(color = colors.onSurfaceVariant))
                    inner()
                }
                if (value.isNotEmpty()) {
                    SymbolIcon(Sym.Close, "清除", Modifier.clip(CircleShape).clickable { onValueChange("") }, size = 18.dp, tint = colors.onSurfaceVariant)
                }
            }
        },
    )
}
