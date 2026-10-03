package top.flysoftbeta.workflow.feature.chat.transcript

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.InlineTextContent
import androidx.compose.foundation.text.appendInlineContent
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.*
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import ru.noties.jlatexmath.JLatexMathDrawable
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Drawable construction is serialized off-main: the upstream parser owns global symbol tables. */
internal object NativeMath {
    private val mutex = Mutex()
    private data class Key(val source: String, val size: Float, val color: Int)
    private val cache = LinkedHashMap<Key, JLatexMathDrawable?>(16, .75f, true)
    suspend fun render(source: String, textSize: Float, color: Int): JLatexMathDrawable? = withContext(Dispatchers.Default) {
        if (!MathSafety.accepts(source)) return@withContext null
        mutex.withLock {
            val key = Key(source, textSize, color)
            if (cache.containsKey(key)) return@withLock cache[key]
            val drawable = try {
                JLatexMathDrawable.builder(source).textSize(textSize).color(color).padding(2).build()
                    .takeIf { it.intrinsicWidth in 1..16384 && it.intrinsicHeight in 1..4096 }
            } catch (_: Exception) { null } catch (_: StackOverflowError) { null }
            cache[key] = drawable
            while (cache.size > 128) cache.remove(cache.keys.first())
            drawable
        }
    }
}

@Composable
internal fun MarkdownBlockContent(block: MarkdownBlock, callbacks: TranscriptCallbacks, modifier: Modifier = Modifier) {
    val colors = WorkflowTheme.colors
    when (block) {
        is MarkdownBlock.Prose -> Row(modifier.fillMaxWidth().padding(start = (block.indent.coerceAtMost(6) * 12).dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (block.quote) Box(Modifier.width(3.dp).height(24.dp).background(colors.outlineVariant))
            block.marker?.let { Text(it, Modifier.widthIn(min = 16.dp), style = WorkflowTheme.text.chat, color = colors.onSurfaceVariant) }
            val style = when (block.heading) { 1 -> WorkflowTheme.text.chatH1; 2 -> WorkflowTheme.text.chatH2; in 3..6 -> WorkflowTheme.text.chatH3; else -> WorkflowTheme.text.chat }
            MarkdownText(block.spans, callbacks, Modifier.weight(1f), style)
        }
        is MarkdownBlock.Code -> Column(modifier.fillMaxWidth().background(colors.surfaceContainerHighest, WorkflowShapes.sm).testTag("native-code")) {
            if (!block.continuation) Row(Modifier.fillMaxWidth().heightIn(min = 28.dp).padding(start = 10.dp), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(block.language.ifEmpty { "代码" }, Modifier.padding(top = 6.dp), style = WorkflowTheme.text.caption, color = colors.onSurfaceVariant)
                TextButton(onClick = { callbacks.copyText(block.copyText) }, contentPadding = PaddingValues(horizontal = 10.dp, vertical = 0.dp)) { Text("复制", style = WorkflowTheme.text.caption) }
            }
            SelectionContainer { Text(block.text, Modifier.horizontalScroll(rememberScrollState()).padding(10.dp), style = WorkflowTheme.text.monoBlock, color = colors.onSurface, softWrap = false) }
        }
        is MarkdownBlock.Formula -> Box(modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 6.dp).testTag("native-display-math")) {
            Formula(block.latex, colors.onSurface)
        }
        is MarkdownBlock.Table -> Column(modifier.horizontalScroll(rememberScrollState()).background(colors.surfaceContainerLow, WorkflowShapes.sm).testTag("native-table")) {
            block.rows.forEachIndexed { row, cells ->
                Row {
                    cells.forEachIndexed { col, spans ->
                        MarkdownText(spans.map { if (row == 0) it.copy(bold = true) else it }, callbacks,
                            Modifier.width(160.dp).padding(horizontal = 10.dp, vertical = 8.dp), WorkflowTheme.text.chat,
                            when (block.alignments.getOrNull(col)) { "RIGHT" -> TextAlign.End; "CENTER" -> TextAlign.Center; else -> TextAlign.Start })
                    }
                }
                if (row != block.rows.lastIndex) HorizontalDivider(color = colors.outlineVariant)
            }
        }
        MarkdownBlock.Rule -> HorizontalDivider(modifier.padding(vertical = 8.dp), color = colors.outlineVariant)
    }
}

@Composable
private fun MarkdownText(spans: List<MarkdownSpan>, callbacks: TranscriptCallbacks, modifier: Modifier, style: TextStyle, alignment: TextAlign = TextAlign.Start) {
    val density = LocalDensity.current
    val colors = WorkflowTheme.colors
    val mono = WorkflowTheme.text.mono.fontFamily
    val px = with(density) { style.fontSize.toPx() }
    val formulae by produceState<Map<Int, JLatexMathDrawable?>>(emptyMap(), spans, px, colors.onSurface) {
        value = buildMap { spans.forEachIndexed { index, span -> if (span.math) put(index, NativeMath.render(span.text, px, colors.onSurface.toArgb())) } }
    }
    BoxWithConstraints(modifier) {
        val content = HashMap<String, InlineTextContent>()
        val annotated = buildAnnotatedString {
            spans.forEachIndexed { index, span ->
                val drawable = formulae[index]
                if (span.math && drawable != null) {
                    val widthPx = minOf(drawable.intrinsicWidth.toFloat(), with(density) { maxWidth.toPx() })
                    val ratio = widthPx / drawable.intrinsicWidth
                    val w = with(density) { widthPx.toSp() }; val h = with(density) { (drawable.intrinsicHeight * ratio).toSp() }
                    content["math$index"] = InlineTextContent(Placeholder(w, h, PlaceholderVerticalAlign.Center)) { MathCanvas(drawable, span.text, Modifier.fillMaxSize()) }
                    appendInlineContent("math$index", "\\(${span.text}\\)")
                } else {
                    val start = length
                    append(if (span.math) "\\(${span.text}\\)" else span.text)
                    addStyle(SpanStyle(fontWeight = if (span.bold) FontWeight.Bold else null, fontStyle = if (span.italic) FontStyle.Italic else null,
                        textDecoration = if (span.strike) TextDecoration.LineThrough else null,
                        fontFamily = if (span.code || span.math) mono else null,
                        color = if (span.link != null) colors.primary else Color.Unspecified,
                        background = if (span.code) colors.surfaceContainerHighest else Color.Unspecified), start, length)
                    span.link?.let { link ->
                        addLink(LinkAnnotation.Clickable(link, TextLinkStyles(SpanStyle(textDecoration = TextDecoration.Underline))) { openLink(link, callbacks) }, start, length)
                    }
                }
            }
        }
        SelectionContainer { Text(annotated, style = style, color = colors.onSurface, inlineContent = content, textAlign = alignment, modifier = Modifier.fillMaxWidth()) }
    }
}

@Composable
private fun Formula(source: String, color: Color) {
    val density = LocalDensity.current
    val px = with(density) { 17.sp.toPx() }
    val drawable by produceState<JLatexMathDrawable?>(null, source, px, color) { value = NativeMath.render(source, px, color.toArgb()) }
    val d = drawable
    if (d != null) MathCanvas(d, source, Modifier.size(with(density) { d.intrinsicWidth.toDp() }, with(density) { d.intrinsicHeight.toDp() }))
    else SelectionContainer { Text("\\[$source\\]", style = WorkflowTheme.text.mono, color = color) }
}

@Composable
private fun MathCanvas(drawable: JLatexMathDrawable, source: String, modifier: Modifier) {
    Canvas(modifier.testTag("native-math").semantics { contentDescription = "公式 $source" }) {
        drawIntoCanvas { canvas ->
            val native = canvas.nativeCanvas
            val save = native.save()
            native.scale(size.width / drawable.intrinsicWidth, size.height / drawable.intrinsicHeight)
            drawable.draw(native)
            native.restoreToCount(save)
        }
    }
}

internal fun openLink(link: String, callbacks: TranscriptCallbacks) {
    val safe = NativeMarkdownParser.safeLink(link) ?: return
    if (safe.substringBefore(':').lowercase() in setOf("http", "https", "mailto")) callbacks.openUrl(safe)
    else callbacks.openPath(safe, null, null)
}
