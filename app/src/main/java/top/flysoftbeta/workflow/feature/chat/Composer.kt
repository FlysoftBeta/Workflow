package top.flysoftbeta.workflow.feature.chat

import android.view.KeyCharacterMap
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.MenuAnchorPosition
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.agent.SendMode
import top.flysoftbeta.workflow.agent.model.LoginState
import top.flysoftbeta.workflow.agent.model.PermissionPreset
import top.flysoftbeta.workflow.agent.model.ProcessState
import top.flysoftbeta.workflow.agent.model.SliderPosition
import top.flysoftbeta.workflow.app.panel.PanelFrame
import top.flysoftbeta.workflow.platform.importer.ImportKind
import top.flysoftbeta.workflow.ui.design.AttachmentItem
import top.flysoftbeta.workflow.ui.design.AttachmentState
import top.flysoftbeta.workflow.ui.design.AttachmentStrip
import top.flysoftbeta.workflow.ui.design.CompositeMenu
import top.flysoftbeta.workflow.ui.design.EffortOption
import top.flysoftbeta.workflow.ui.design.ItemListMenu
import top.flysoftbeta.workflow.ui.design.ListMenuItem
import top.flysoftbeta.workflow.ui.design.MenuEntry
import top.flysoftbeta.workflow.ui.design.MenuGroup
import top.flysoftbeta.workflow.ui.design.ModelChip
import top.flysoftbeta.workflow.ui.design.ModelEffortPopover
import top.flysoftbeta.workflow.ui.design.ModelEffortSelection
import top.flysoftbeta.workflow.ui.design.ModelOption
import top.flysoftbeta.workflow.ui.design.SymbolIcon
import top.flysoftbeta.workflow.ui.design.WfIconButton
import top.flysoftbeta.workflow.ui.design.icons.Sym
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme
import top.flysoftbeta.workflow.agent.model.ModelCatalog as AgentCatalog

/** docs/ui.md §4.8: surfaceContainerHigh card, xl corners, 8dp margin, above the IME. */
@Composable
internal fun Composer(c: ConversationController, frame: PanelFrame, modifier: Modifier) {
    val colors = WorkflowTheme.colors
    val model = c.composer
    val focus = remember { FocusRequester() }
    LaunchedEffect(c) { c.focusRequests.collect { n -> if (n > 0) runCatching { focus.requestFocus() } } }
    Surface(modifier, shape = WorkflowShapes.xl, color = colors.surfaceContainerHigh) {
        Column(Modifier.padding(top = 4.dp)) {
            val queued = c.queued
            if (queued.isNotEmpty()) QueuedChips(c)
            if (model.attachments.isNotEmpty()) {
                AttachmentStrip(
                    items = model.attachments.map { a ->
                        AttachmentItem(a.key, a.name, a.image, thumbnail = a.thumbnail, state = when {
                            a.failed -> AttachmentState.Failed
                            a.path == null -> AttachmentState.Uploading(null)
                            else -> AttachmentState.Ready
                        })
                    },
                    onRemove = model::remove,
                    onOpen = { key -> model.attachments.firstOrNull { it.key == key }?.path?.let { c.context.commands.openFile(it) } },
                )
            }
            val hardwareEnter = Modifier.onPreviewKeyEvent { event ->
                val enter = event.key == Key.Enter || event.key == Key.NumPadEnter
                if (!enter || event.type != KeyEventType.KeyDown || event.isShiftPressed) return@onPreviewKeyEvent false
                // Soft keyboards insert a newline; only a physical keyboard's Enter sends.
                if (event.nativeKeyEvent.deviceId == KeyCharacterMap.VIRTUAL_KEYBOARD || event.nativeKeyEvent.device?.isVirtual == true) return@onPreviewKeyEvent false
                if (canSend(c)) c.send(if (c.running) SendMode.STEER else SendMode.AUTO)
                true
            }
            Box(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
                if (model.text.text.isEmpty()) {
                    Text(placeholder(c), style = WorkflowTheme.text.chat, color = colors.onSurfaceVariant)
                }
                BasicTextField(
                    value = model.text,
                    onValueChange = model::onTextChange,
                    modifier = Modifier.fillMaxWidth().focusRequester(focus).then(hardwareEnter).semantics { contentDescription = "消息" },
                    textStyle = WorkflowTheme.text.chat.copy(color = colors.onSurface),
                    cursorBrush = SolidColor(colors.primary),
                    minLines = 1,
                    maxLines = 8,
                )
            }
            Row(Modifier.fillMaxWidth().height(40.dp).padding(horizontal = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                AddMenu(c)
                PermissionChip(c)
                Spacer(Modifier.weight(1f))
                ModelControl(c)
                Spacer(Modifier.width(6.dp))
                SendButton(c)
            }
            Spacer(Modifier.height(4.dp))
        }
    }
}

private fun placeholder(c: ConversationController): String = when {
    c.running -> "继续输入以引导或排队"
    else -> "随心输入"
}

/** Whether the backend can take a message now (ui.md §4.8: disabled when not logged in / no model). */
internal fun backendReady(c: ConversationController): Boolean {
    val backend = c.backend ?: return false
    if (!c.available || c.reviewerBlocked) return false
    if (backend.account.state == LoginState.LOGGED_OUT || backend.account.state == LoginState.LOGGING_IN) return false
    return backend.process is ProcessState.Ready && c.selection != null
}

private fun canSend(c: ConversationController) = backendReady(c) && c.composer.hasContent && !c.composer.importing

@Composable
private fun QueuedChips(c: ConversationController) {
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 12.dp, vertical = 4.dp)) {
        c.queued.forEach { turn ->
            val text = turn.items.filterIsInstance<top.flysoftbeta.workflow.agent.model.UserMessageItem>().firstOrNull()?.text.orEmpty()
            Row(
                Modifier.padding(end = 6.dp).height(32.dp).clip(WorkflowShapes.sm).background(WorkflowTheme.colors.surfaceContainerHighest).padding(start = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                SymbolIcon(Sym.Schedule, null, size = 16.dp, tint = WorkflowTheme.colors.onSurfaceVariant)
                Spacer(Modifier.width(6.dp))
                Text(text.lineSequence().first().ifEmpty { "附件" }, Modifier.widthIn(max = 200.dp), style = WorkflowTheme.text.label,
                    color = WorkflowTheme.colors.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                WfIconButton(Sym.Close, "移出队列", { c.cancelQueued(turn) }, iconSize = 16.dp)
            }
        }
    }
}

/** "+": 拍照 · 相册 · 设备文件 ┆ 工作区文件… — an anchored menu, not a sheet (ui.md §4.8). */
@Composable
private fun AddMenu(c: ConversationController) {
    var open by remember { mutableStateOf(false) }
    var picking by remember { mutableStateOf(false) }
    Box {
        WfIconButton(Sym.Add, "添加", { open = true })
        CompositeMenu(
            expanded = open, onDismissRequest = { open = false }, anchorPosition = MenuAnchorPosition.Above,
            groups = listOf(
                MenuGroup("device", listOf(
                    MenuEntry.Action("camera", "拍照", Sym.PhotoCamera) { c.pick(ImportKind.CAMERA) },
                    MenuEntry.Action("gallery", "相册", Sym.PhotoLibrary) { c.pick(ImportKind.GALLERY) },
                    MenuEntry.Action("files", "设备文件", Sym.UploadFile) { c.pick(ImportKind.FILES) },
                )),
                MenuGroup("workspace", listOf(MenuEntry.Action("workspace", "工作区文件…", Sym.AccountTree) { picking = true })),
            ),
        )
    }
    if (picking) WorkspaceFilePicker(c.context.store, onDismiss = { picking = false }) { paths ->
        picking = false
        c.composer.addPaths(paths)
    }
}

@Composable
private fun PermissionChip(c: ConversationController) {
    var open by remember { mutableStateOf(false) }
    val colors = WorkflowTheme.colors
    Box {
        Row(
            Modifier.height(32.dp).clip(WorkflowShapes.full).clickable { open = true }.padding(horizontal = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SymbolIcon(if (c.permissions == PermissionPreset.AUTO_EDIT) Sym.EditDocument else Sym.Shield, null, size = 16.dp, tint = colors.onSurfaceVariant)
            Spacer(Modifier.width(4.dp))
            Text(ChatText.permissionLabel(c.permissions), style = WorkflowTheme.text.label, color = colors.onSurfaceVariant)
        }
        CompositeMenu(
            expanded = open, onDismissRequest = { open = false }, anchorPosition = MenuAnchorPosition.Above,
            groups = listOf(MenuGroup("permissions", PermissionPreset.entries.map { preset ->
                MenuEntry.Action(preset.name, ChatText.permissionLabel(preset) + " · " + ChatText.permissionDescription(preset),
                    if (preset == c.permissions) Sym.Check else null) { c.setPermissionPreset(preset) }
            })),
        )
    }
}

/** The one model + effort control (ui.md §4.8): chip → popover slider; › → the full model list. */
@Composable
private fun ModelControl(c: ConversationController) {
    val catalog = c.backend?.models
    val selection = c.selection
    var open by remember { mutableStateOf(false) }
    var list by remember { mutableStateOf(false) }
    if (catalog == null || catalog.models.isEmpty() || selection == null) {
        ModelChip("选择模型", onClick = {}, enabled = false)
        return
    }
    val current = catalog.model(selection.model)
    val label = listOfNotNull(current?.displayName ?: selection.model, selection.effort?.let(ChatText::effortLabel)).joinToString(" ")
    val models = sliderModels(catalog, selection)
    Box {
        ModelChip(label, onClick = { open = true })
        ModelEffortPopover(
            expanded = open,
            onDismissRequest = { open = false },
            models = models,
            selection = ModelEffortSelection(selection.model, selection.effort ?: ""),
            onSelectionChange = { s -> c.select(SliderPosition(s.modelId, s.effortId.ifEmpty { null })) },
            onOpenModelList = { open = false; list = true },
        )
        ItemListMenu(
            expanded = list, onDismissRequest = { list = false },
            items = catalog.models.filter { !it.hidden || it.id == selection.model }.map { m ->
                ListMenuItem(m.id, m.displayName, caption = m.description?.lineSequence()?.firstOrNull())
            },
            selectedKey = selection.model,
            onSelect = { id ->
                list = false
                catalog.resolve(SliderPosition(id, null))?.let(c::select)
                open = true
            },
            anchorPosition = MenuAnchorPosition.Above,
        )
    }
}

/** At most 4 segments: the current model, the default, then the others in backend order. */
internal fun sliderModels(catalog: AgentCatalog, selection: SliderPosition): List<ModelOption> {
    val segments = catalog.slider(selection.model)
    val chosen = LinkedHashSet<String>()
    chosen += selection.model
    catalog.defaultModel?.let { chosen += it.id }
    segments.forEach { if (chosen.size < 4) chosen += it.model.id }
    return segments.filter { it.model.id in chosen }.map { segment ->
        val efforts = segment.detents.map { EffortOption(it, ChatText.effortLabel(it)) }.ifEmpty { listOf(EffortOption("", "默认")) }
        ModelOption(segment.model.id, segment.model.displayName, efforts)
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun SendButton(c: ConversationController) {
    val colors = WorkflowTheme.colors
    val hasText = c.composer.hasContent
    val ready = backendReady(c)
    var queueMenu by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically) {
        if (c.running) {
            RoundButton(Sym.Stop, "停止", enabled = true, primary = !hasText) { c.stop() }
            if (hasText) {
                Spacer(Modifier.width(6.dp))
                Box {
                    RoundButton(Sym.ArrowUpward, "引导", enabled = ready && !c.composer.importing, primary = true,
                        onLongClick = { queueMenu = true }) { c.send(SendMode.STEER) }
                    CompositeMenu(
                        expanded = queueMenu, onDismissRequest = { queueMenu = false }, anchorPosition = MenuAnchorPosition.Above,
                        groups = listOf(MenuGroup("send", listOf(
                            MenuEntry.Action("steer", "立即引导", Sym.ArrowUpward) { c.send(SendMode.STEER) },
                            MenuEntry.Action("queue", "排队到下一轮", Sym.Schedule) { c.send(SendMode.QUEUE) },
                        ))),
                    )
                }
            }
        } else {
            RoundButton(Sym.ArrowUpward, "发送", enabled = ready && hasText && !c.composer.importing, primary = true) { c.send() }
        }
    }
    @Suppress("UNUSED_EXPRESSION") colors
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun RoundButton(icon: Int, description: String, enabled: Boolean, primary: Boolean, onLongClick: (() -> Unit)? = null, onClick: () -> Unit) {
    val colors = WorkflowTheme.colors
    val background = when {
        !enabled -> colors.onSurface.copy(alpha = 0.12f)
        primary -> colors.primary
        else -> colors.surfaceContainerHighest
    }
    val tint = when {
        !enabled -> colors.onSurface.copy(alpha = 0.38f)
        primary -> colors.onPrimary
        else -> colors.onSurface
    }
    Box(
        Modifier
            .size(36.dp)
            .clip(WorkflowShapes.full)
            .background(background)
            .combinedClickable(enabled = enabled, onClick = onClick, onLongClick = onLongClick, onClickLabel = description)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) { SymbolIcon(icon, null, size = 20.dp, tint = tint) }
}
