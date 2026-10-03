package top.flysoftbeta.workflow.ui.design

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.time.DayOfWeek
import java.time.LocalDate
import java.time.temporal.TemporalAdjusters
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/**
 * Age buckets of the temporary-session timeline (docs/ux/README.md §4.9). Older entries "blur" by lower
 * opacity, lighter weight and fewer resources. The 0.62 floor keeps ≥ 3:1 contrast (§7).
 */
enum class TimelineAge(val label: String, val alpha: Float, val weight: FontWeight, val maxResources: Int) {
    Today("今天", 1.0f, FontWeight.Medium, 3),
    Yesterday("昨天", 0.85f, FontWeight.Normal, 2),
    ThisWeek("本周", 0.72f, FontWeight.Normal, 1),
    Older("更早", 0.62f, FontWeight.Normal, 1);

    companion object {
        /** Bucket of [date] relative to [today]; "this week" is the ISO week (Monday first) before yesterday. */
        fun of(date: LocalDate, today: LocalDate): TimelineAge {
            val yesterday = today.minusDays(1)
            val weekStart = today.with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY))
            return when {
                !date.isBefore(today) -> Today
                date == yesterday -> Yesterday
                !date.isBefore(weekStart) -> ThisWeek
                else -> Older
            }
        }
    }
}

/** 28dp caption group header (今天 / 昨天 / …), shared by timelines and time-grouped lists. */
@Composable
fun GroupHeader(label: String, modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(WorkflowTheme.dimens.groupHeader).padding(horizontal = WorkflowTheme.dimens.padH), contentAlignment = Alignment.CenterStart) {
        Text(label, style = WorkflowTheme.text.caption, color = WorkflowTheme.colors.onSurfaceVariant)
    }
}

/**
 * One timeline node: a 1dp rail with a node dot, the start [time] and the main [resources]
 * (trimmed to the age's budget; [TimelineAge.Older] shows only the first). No "临时会话" label,
 * no panel count. [isFirst]/[isLast] end the rail at the node.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun TimelineEntry(
    time: String,
    resources: List<String>,
    age: TimelineAge,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    current: Boolean = false,
    isFirst: Boolean = false,
    isLast: Boolean = false,
    onLongClick: (() -> Unit)? = null,
) {
    val colors = WorkflowTheme.colors
    val text = WorkflowTheme.text
    val haptics = LocalHapticFeedback.current
    val shown = resources.take(age.maxResources)
    Row(
        modifier
            .fillMaxWidth()
            .height(IntrinsicSize.Min)
            .heightIn(min = WorkflowTheme.dimens.listRow)
            .combinedClickable(
                onClick = onClick,
                onLongClick = onLongClick?.let { { haptics.performHapticFeedback(HapticFeedbackType.LongPress); it() } },
            )
            .alpha(age.alpha)
            .padding(end = WorkflowTheme.dimens.padH),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val rail = colors.outlineVariant
        Box(
            Modifier
                .width(32.dp)
                .fillMaxHeight()
                .drawBehind {
                    val x = size.width / 2
                    val top = if (isFirst) size.height / 2 else 0f
                    val bottom = if (isLast) size.height / 2 else size.height
                    drawLine(rail, Offset(x, top), Offset(x, bottom), strokeWidth = 1.dp.toPx())
                },
            contentAlignment = Alignment.Center,
        ) {
            Box(Modifier.size(if (current) 9.dp else 7.dp).background(if (current) colors.primary else colors.outline, CircleShape))
        }
        Column(Modifier.weight(1f).padding(vertical = 6.dp)) {
            Text(time, style = text.label.copy(fontWeight = age.weight), color = colors.onSurface, maxLines = 1)
            if (shown.isNotEmpty()) {
                Text(
                    shown.joinToString(" · "), style = text.caption.copy(fontWeight = age.weight),
                    color = colors.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

/**
 * Hairline-flanked centred caption: system notices in the transcript (quota, compaction) and the
 * 30-minute time separators (docs/ux/README.md §4.5).
 */
@Composable
fun HairlineCaption(text: String, modifier: Modifier = Modifier, trailing: (@Composable () -> Unit)? = null) {
    val colors = WorkflowTheme.colors
    Row(modifier.fillMaxWidth().padding(vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.weight(1f).height(1.dp).background(colors.outlineVariant))
        Spacer(Modifier.width(12.dp))
        Text(text, style = WorkflowTheme.text.caption, color = colors.onSurfaceVariant)
        if (trailing != null) { Spacer(Modifier.width(6.dp)); trailing() }
        Spacer(Modifier.width(12.dp))
        Box(Modifier.weight(1f).height(1.dp).background(colors.outlineVariant))
    }
}
