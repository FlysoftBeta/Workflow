package top.flysoftbeta.workflow.feature.sessions

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.app.Shell
import top.flysoftbeta.workflow.app.clockLabel
import top.flysoftbeta.workflow.app.relativeTime
import top.flysoftbeta.workflow.app.sessionLabel
import top.flysoftbeta.workflow.core.session.Session
import top.flysoftbeta.workflow.core.session.SessionKind
import top.flysoftbeta.workflow.core.session.SessionPolicy
import top.flysoftbeta.workflow.core.session.TimelineBucket
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.EmptyState
import top.flysoftbeta.workflow.ui.design.TextAction
import top.flysoftbeta.workflow.ui.design.GroupHeader
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.SearchField
import top.flysoftbeta.workflow.ui.design.StatusDot
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.TimelineAge
import top.flysoftbeta.workflow.ui.design.TimelineEntry
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import java.time.Instant
import java.time.ZoneId

/**
 * The session view (product.md §3, ui.md §4.9): a 400dp modal side sheet from the left (full screen
 * below 600dp) with search, the ranked persistent sessions, the temporary timeline that fades with age,
 * and the archived list. Tap switches, long press names or archives.
 */
@Composable
fun SessionsSheet(shell: Shell, modifier: Modifier = Modifier) {
    val open = shell.sessionsOpen
    val motion = WorkflowTheme.motion
    BoxWithConstraints(modifier.fillMaxSize()) {
        val wide = maxWidth >= 600.dp
        AnimatedVisibility(open, enter = fadeIn(motion.fastEffectsSpec()), exit = fadeOut(motion.fastEffectsSpec())) {
            Box(
                Modifier
                    .fillMaxSize()
                    .background(WorkflowTheme.colors.scrim.copy(alpha = 0.32f))
                    .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) { shell.sessionsOpen = false },
            )
        }
        AnimatedVisibility(
            open,
            enter = slideInHorizontally(motion.defaultSpatialSpec()) { -it },
            exit = slideOutHorizontally(motion.defaultSpatialSpec()) { -it },
        ) {
            Surface(
                if (wide) Modifier.width(400.dp).fillMaxHeight() else Modifier.fillMaxSize(),
                color = WorkflowTheme.colors.surfaceContainerLow,
                shadowElevation = 3.dp,
            ) { SessionsContent(shell) }
        }
    }
}

@Composable
private fun SessionsContent(shell: Shell) {
    val state by shell.store.state.collectAsState()
    var query by remember { mutableStateOf("") }
    var archivedView by remember { mutableStateOf(false) }
    val zone = remember { ZoneId.systemDefault() }
    val now = remember(state.sessions) { System.currentTimeMillis() }
    val current = state.activeSessionId

    BackHandler(enabled = true) { if (archivedView) archivedView = false else shell.sessionsOpen = false }

    Column(Modifier.fillMaxSize()) {
        Row(
            Modifier.fillMaxWidth().height(44.dp).padding(start = 4.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (archivedView) WfIconButton(Sym.ArrowBack, "返回", { archivedView = false })
            SearchField(query, { query = it }, Modifier.weight(1f).padding(horizontal = 4.dp), placeholder = "搜索会话")
            WfIconButton(Sym.Add, "新建 Session", { shell.newSession() })
        }
        val titleOf = { ref: top.flysoftbeta.workflow.core.resource.ResourceRef -> shell.resourceTitle(ref) }
        LazyColumn(Modifier.fillMaxWidth().weight(1f)) {
            when {
                query.isNotBlank() -> {
                    val matches = SessionPolicy.search(state.sessions, query, titleOf)
                    if (matches.isEmpty()) item(key = "no-matches") {
                        EmptyState(listOf(TextAction("清除搜索") { query = "" }), Modifier.fillMaxWidth().height(180.dp), "没有匹配的会话")
                    }
                    items(matches, key = { "m:" + it.session.id }) { match ->
                        SessionRow(shell, match.session, current, now, zone, archived = match.session.isArchived)
                    }
                }
                archivedView -> {
                    val archived = state.archivedSessions.sortedByDescending { it.archivedAt ?: it.lastUsedAt }
                    if (archived.isEmpty()) item(key = "no-archives") {
                        EmptyState(listOf(TextAction("返回会话") { archivedView = false }), Modifier.fillMaxWidth().height(180.dp), "没有已归档会话")
                    }
                    items(archived, key = { "a:" + it.id }) { SessionRow(shell, it, current, now, zone, archived = true) }
                }
                else -> {
                    val ranking = SessionPolicy.persistentRanking(state.sessions, state.pinned, now)
                    val persistent = ranking.pinned + ranking.ranked
                    items(persistent, key = { "p:" + it.id }) { SessionRow(shell, it, current, now, zone, archived = false) }
                    val groups = SessionPolicy.timeline(state.sessions, now, zone)
                    groups.forEach { group ->
                        item(key = "g:" + group.bucket.name) { GroupHeader(bucketLabel(group.bucket)) }
                        items(group.sessions.size, key = { "t:" + group.sessions[it].id }) { index ->
                            val session = group.sessions[index]
                            TimelineRow(shell, session, group.bucket, current == session.id, index == 0, index == group.sessions.lastIndex, zone, now)
                        }
                    }
                }
            }
        }
        if (!archivedView && query.isBlank()) {
            HorizontalDivider(color = WorkflowTheme.colors.outlineVariant)
            Row(
                Modifier
                    .fillMaxWidth()
                    .heightIn(min = WorkflowTheme.dimens.listRow)
                    .clickable { archivedView = true }
                    .padding(horizontal = WorkflowTheme.dimens.padH),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("已归档 (${state.archivedSessions.size})", Modifier.weight(1f), style = WorkflowTheme.text.body, color = WorkflowTheme.colors.onSurfaceVariant)
                SymbolIcon(Sym.ChevronRight, null, size = 18.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
            }
        }
    }
}

private fun bucketLabel(bucket: TimelineBucket) = when (bucket) {
    TimelineBucket.TODAY -> "今天"
    TimelineBucket.YESTERDAY -> "昨天"
    TimelineBucket.THIS_WEEK -> "本周"
    TimelineBucket.EARLIER -> "更早"
}

private fun bucketAge(bucket: TimelineBucket) = when (bucket) {
    TimelineBucket.TODAY -> TimelineAge.Today
    TimelineBucket.YESTERDAY -> TimelineAge.Yesterday
    TimelineBucket.THIS_WEEK -> TimelineAge.ThisWeek
    TimelineBucket.EARLIER -> TimelineAge.Older
}

private fun resourceNames(shell: Shell, session: Session): List<String> =
    session.workbench.primaryResources.map { shell.resourceTitle(it) }

/** Temporary session on the timeline: start time and main resources, fading with age. */
@Composable
private fun TimelineRow(shell: Shell, session: Session, bucket: TimelineBucket, current: Boolean, first: Boolean, last: Boolean, zone: ZoneId, now: Long) {
    var menu by remember { mutableStateOf(false) }
    Box {
        val start = Instant.ofEpochMilli(session.createdAt).atZone(zone)
        val time = if (bucket == TimelineBucket.TODAY) "%02d:%02d".format(start.hour, start.minute) else clockLabel(session.createdAt, zone, now)
        TimelineEntry(
            time = time,
            resources = resourceNames(shell, session),
            age = bucketAge(bucket),
            onClick = { shell.switchTo(session.id) },
            current = current,
            isFirst = first,
            isLast = last,
            onLongClick = { menu = true },
        )
        SessionMenu(shell, session, archived = false, expanded = menu) { menu = false }
    }
}

/** Named (or archived / matched) session: two-line row, name + resources, relative time. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun SessionRow(shell: Shell, session: Session, current: String?, now: Long, zone: ZoneId, archived: Boolean) {
    var menu by remember { mutableStateOf(false) }
    val haptics = LocalHapticFeedback.current
    val colors = WorkflowTheme.colors
    val text = WorkflowTheme.text
    Box {
        Row(
            Modifier
                .fillMaxWidth()
                .heightIn(min = WorkflowTheme.dimens.listRowTwoLine)
                .combinedClickable(
                    onClick = { shell.switchTo(session.id) },
                    onLongClick = { haptics.performHapticFeedback(HapticFeedbackType.LongPress); menu = true },
                )
                .padding(start = WorkflowTheme.dimens.padH, end = if (archived) 4.dp else WorkflowTheme.dimens.padH, top = 6.dp, bottom = 6.dp)
                .semantics { if (session.id == current) contentDescription = "当前会话" },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        sessionLabel(session, zone, now), Modifier.weight(1f, fill = false),
                        style = text.titleSm, color = colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    )
                    if (session.id == current) {
                        Spacer(Modifier.width(6.dp))
                        StatusDot(colors.primary)
                    }
                }
                val resources = resourceNames(shell, session)
                if (resources.isNotEmpty()) {
                    Text(resources.take(3).joinToString(" · "), style = text.caption, color = colors.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
            Spacer(Modifier.width(8.dp))
            if (archived) {
                TextButton(onClick = { shell.switchTo(session.id) }) { Text("恢复") }
            } else {
                Text(relativeTime(session.lastUsedAt, zone, now), style = text.caption, color = colors.onSurfaceVariant)
            }
        }
        SessionMenu(shell, session, archived, menu) { menu = false }
    }
}

/** Long-press menu of a session row: 命名 / 重命名 · 归档…（or 恢复 for archived ones）. */
@Composable
private fun SessionMenu(shell: Shell, session: Session, archived: Boolean, expanded: Boolean, onDismiss: () -> Unit) {
    val entries = buildList {
        add(MenuEntry.Action("name", if (session.kind == SessionKind.TEMPORARY) "命名…" else "重命名…", Sym.Edit) { shell.renaming = session.id })
        if (archived) add(MenuEntry.Action("restore", "恢复", Sym.Unarchive) { shell.switchTo(session.id) })
        else add(MenuEntry.Action("archive", "归档…", Sym.Archive) { shell.archive(session.id) })
    }
    CompositeMenu(expanded = expanded, onDismissRequest = onDismiss, groups = listOf(MenuGroup("session", entries)))
}
