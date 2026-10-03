package top.flysoftbeta.workflow.ui.design.gallery

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.StackTabBar
import top.flysoftbeta.workflow.ui.design.TabItem
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.ThemeMode
import top.flysoftbeta.workflow.ui.design.theme.UiDensity
import top.flysoftbeta.workflow.ui.design.theme.WorkflowDesignTheme
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.ui.design.theme.frame

/** Root of the debug gallery: a page tab row (itself a [StackTabBar]) + theme/density toggles. */
@Composable
fun DesignGallery(initial: GalleryConfig) {
    var page by remember { mutableStateOf(initial.page) }
    var theme by remember { mutableStateOf(initial.theme) }
    var density by remember { mutableStateOf(initial.density) }
    WorkflowDesignTheme(themeMode = theme, density = density) {
        top.flysoftbeta.workflow.ui.design.theme.WorkflowSystemBarsEffect()
        Column(Modifier.fillMaxSize().background(WorkflowTheme.colors.frame).safeDrawingPadding()) {
            StackTabBar(
                tabs = GalleryPage.entries.map { TabItem(it.name, it.title, Sym.Apps) },
                activeKey = page.name,
                onSelect = { key -> page = GalleryPage.valueOf(key) },
                onClose = {},
                trailing = {
                    WfIconButton(
                        if (theme == ThemeMode.Dark) Sym.LightMode else Sym.Contrast, "切换主题",
                        { theme = if (theme == ThemeMode.Dark) ThemeMode.Light else ThemeMode.Dark },
                    )
                    WfIconButton(
                        Sym.FitScreen, if (density == UiDensity.Compact) "标准密度" else "紧凑密度",
                        { density = if (density == UiDensity.Compact) UiDensity.Standard else UiDensity.Compact },
                        checked = density == UiDensity.Standard,
                    )
                },
            )
            Box(Modifier.weight(1f).fillMaxWidth()) {
                when (page) {
                    GalleryPage.Chrome -> ChromePage()
                    GalleryPage.Menus -> MenusPage()
                    GalleryPage.Content -> ContentPage()
                    GalleryPage.Layout -> LayoutDndPage(initial.dnd)
                    GalleryPage.Tokens -> TokensPage()
                }
            }
        }
    }
}

/** Section label used inside gallery pages (gallery-only; product UI has no such headings). */
@Composable
internal fun GallerySection(title: String, modifier: Modifier = Modifier, content: @Composable () -> Unit) {
    Column(modifier.padding(bottom = 12.dp)) {
        androidx.compose.material3.Text(
            title, Modifier.padding(start = 4.dp, bottom = 4.dp),
            style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.primary,
        )
        content()
    }
}

@Composable
internal fun FakeLines(lines: List<String>, modifier: Modifier = Modifier, mono: Boolean = true) {
    Column(modifier.padding(horizontal = 8.dp, vertical = 6.dp)) {
        lines.forEachIndexed { index, line ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (mono) {
                    androidx.compose.material3.Text(
                        "${index + 1}".padStart(3), Modifier.width(32.dp), style = WorkflowTheme.text.mono,
                        color = WorkflowTheme.colors.onSurfaceVariant,
                    )
                }
                androidx.compose.material3.Text(
                    line, style = if (mono) WorkflowTheme.text.mono else WorkflowTheme.text.body,
                    color = WorkflowTheme.colors.onSurface, maxLines = 1,
                )
            }
        }
    }
}

/** A file-tree-like row for demos (the real tree belongs to the files workstream). */
@Composable
internal fun DemoTreeRow(name: String, depth: Int, icon: Int, selected: Boolean = false, dirty: Boolean = false, open: Boolean = false, onClick: () -> Unit = {}) {
    val dimens = WorkflowTheme.dimens
    val colors = WorkflowTheme.colors
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = dimens.treeRow)
            .padding(horizontal = 4.dp)
            .background(if (selected) colors.secondaryContainer else colors.surface, top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes.sm)
            .clickable(onClick = onClick)
            .padding(start = 4.dp + dimens.indent * depth, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        top.flysoftbeta.workflow.ui.design.SymbolIcon(icon, null, size = 16.dp, tint = colors.onSurfaceVariant)
        Spacer(Modifier.width(6.dp))
        androidx.compose.material3.Text(
            name, Modifier.weight(1f), style = if (open) WorkflowTheme.text.labelActive else WorkflowTheme.text.label,
            color = colors.onSurface, maxLines = 1,
        )
        if (dirty) top.flysoftbeta.workflow.ui.design.StatusDot(colors.primary, size = 6.dp)
    }
}

internal val demoCode = listOf(
    "package top.flysoftbeta.workflow.ui.design",
    "",
    "/** Dimension tokens (docs/ui.md §1.1). */",
    "@Immutable",
    "data class WorkflowDimens(",
    "    val density: UiDensity,",
    "    val bar: Dp,",
    "    val iconButton: Dp,",
    "    val iconButtonTouch: Dp,",
    "    val treeRow: Dp,",
    ")",
    "",
    "val LocalWorkflowDimens = staticCompositionLocalOf { WorkflowDimens.Compact }",
    "// 中文注释与 CJK 回退：设计系统",
)

internal val demoTerminal = listOf(
    "work@workflow:/workspace$ git status --short",
    " M app/src/main/java/top/flysoftbeta/workflow/ui/design/StackTabBar.kt",
    "?? app/src/debug/",
    "work@workflow:/workspace$ ./gradlew :app:assembleDebug",
    "BUILD SUCCESSFUL in 41s",
    "work@workflow:/workspace$ ",
)
