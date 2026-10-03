package top.flysoftbeta.workflow.ui.design.gallery

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Color roles, type scale, radii, density tokens and every bundled glyph. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TokensPage() {
    val c = WorkflowTheme.colors
    val e = WorkflowTheme.extendedColors
    val t = WorkflowTheme.text
    val d = WorkflowTheme.dimens
    Row(Modifier.fillMaxSize().padding(8.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
            GallerySection("Color roles (generated, SchemeTonalSpot #286B57)") {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    listOf(
                        "primary" to c.primary, "onPrimary" to c.onPrimary, "primaryContainer" to c.primaryContainer,
                        "secondaryContainer" to c.secondaryContainer, "tertiary" to c.tertiary, "tertiaryContainer" to c.tertiaryContainer,
                        "surface" to c.surface, "surfaceContainerLow" to c.surfaceContainerLow, "surfaceContainer (frame)" to c.surfaceContainer,
                        "surfaceContainerHigh" to c.surfaceContainerHigh, "surfaceContainerHighest" to c.surfaceContainerHighest,
                        "onSurface" to c.onSurface, "onSurfaceVariant" to c.onSurfaceVariant, "outline" to c.outline,
                        "outlineVariant" to c.outlineVariant, "error" to c.error, "errorContainer" to c.errorContainer,
                        "success" to e.success, "successContainer" to e.successContainer, "warning" to e.warning, "warningContainer" to e.warningContainer,
                    ).forEach { (name, color) -> Swatch(name, color) }
                }
            }
            GallerySection("Type scale") {
                listOf(
                    "titleMd 16/22 500 对话框标题" to t.titleMd, "titleSm 14/20 500 会话名" to t.titleSm,
                    "body 14/20 列表与菜单" to t.body, "label 13/18 Tab 与树" to t.label, "labelActive 13/18 500" to t.labelActive,
                    "caption 12/16 时间 次要信息" to t.caption, "micro 11/14 刻度 徽标" to t.micro,
                    "chat 15/24 消息正文" to t.chat, "h1 20/28 600" to t.chatH1, "h2 18/26 600" to t.chatH2, "h3 16/24 600" to t.chatH3,
                    "mono 13/19 val x = 0 // 等宽" to t.mono, "monoBlock 13/20 fun f() = 1" to t.monoBlock,
                ).forEach { (label, style) -> TypeSample(label, style) }
            }
        }
        Column(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
            GallerySection("Radii xs 4 · sm 8 · md 12 · lg 20 · xl 24 · full") {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    listOf(WorkflowShapes.xs, WorkflowShapes.sm, WorkflowShapes.md, WorkflowShapes.lg, WorkflowShapes.xl, WorkflowShapes.full).forEach {
                        Box(Modifier.size(48.dp).background(c.primaryContainer, it))
                    }
                }
            }
            GallerySection("Density: ${d.density}") {
                Text(
                    "bar ${d.bar} · iconBtn ${d.iconButton}/${d.iconButtonTouch} · icon ${d.icon}/${d.iconSmall} · treeRow ${d.treeRow} · " +
                        "listRow ${d.listRow}/${d.listRowTwoLine} · menuItem ${d.menuItem} · extraKeys ${d.extraKeysRow}/${d.extraKey} · " +
                        "gap ${d.gap} · padH ${d.padH} · indent ${d.indent}",
                    style = t.caption, color = c.onSurface,
                )
                Row(Modifier.padding(top = 6.dp), verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    TokenBar("bar", d.bar); TokenBar("treeRow", d.treeRow); TokenBar("menuItem", d.menuItem)
                    TokenBar("listRow", d.listRow); TokenBar("list2", d.listRowTwoLine); TokenBar("extraKey", d.extraKey)
                }
            }
            GallerySection("Material Symbols Rounded (${Sym.all.size} glyphs, opsz 20)") {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(2.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Sym.all.forEach { (_, id) ->
                        Box(Modifier.size(32.dp).background(c.surface, WorkflowShapes.xs), contentAlignment = Alignment.Center) {
                            SymbolIcon(id, null, tint = c.onSurface)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun Swatch(name: String, color: Color) {
    Box(
        Modifier.width(132.dp).height(44.dp).background(color, WorkflowShapes.sm).border(1.dp, WorkflowTheme.colors.outlineVariant, WorkflowShapes.sm).padding(6.dp),
        contentAlignment = Alignment.BottomStart,
    ) {
        Text(name, style = WorkflowTheme.text.micro, color = if (color.luminance() > 0.4f) Color.Black else Color.White, maxLines = 2)
    }
}

@Composable
private fun TypeSample(label: String, style: TextStyle) {
    Text(label, Modifier.padding(vertical = 2.dp), style = style, color = WorkflowTheme.colors.onSurface, maxLines = 1)
}

@Composable
private fun TokenBar(label: String, height: androidx.compose.ui.unit.Dp) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.width(56.dp).height(height).background(WorkflowTheme.colors.secondaryContainer, WorkflowShapes.xs), contentAlignment = Alignment.Center) {
            Text("$height", style = WorkflowTheme.text.micro, color = WorkflowTheme.colors.onSecondaryContainer)
        }
        Text(label, style = WorkflowTheme.text.micro, color = WorkflowTheme.colors.onSurfaceVariant)
    }
}
