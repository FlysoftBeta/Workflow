package top.flysoftbeta.workflow.ui.design

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import top.flysoftbeta.workflow.ui.design.theme.WorkflowShapes
import top.flysoftbeta.workflow.ui.design.theme.WorkflowTheme

/** Named non-character keys emitted by the extra-keys row. */
enum class SpecialKey { Escape, Tab, Left, Down, Up, Right, Home, End, PageUp, PageDown, Delete, Enter, Backspace,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12 }

enum class ModifierKey { Ctrl, Alt }

/** Latching state of a modifier (docs/ui.md §4.4): tap = one shot, double tap = locked, tap again = off. */
enum class Latch { Off, OneShot, Locked }

/** One key of the row. */
@Immutable
sealed interface ExtraKey {
    val label: String
    val description: String get() = label

    data class Special(val key: SpecialKey, override val label: String, val repeat: Boolean = false) : ExtraKey
    /** Literal text (| ~ / - _ brackets). */
    data class Text(val text: String, override val label: String = text) : ExtraKey
    /** A control chord such as ^C (sent as Ctrl + [char]). */
    data class Control(val char: Char, override val label: String = "^$char") : ExtraKey
    data class Latching(val modifier: ModifierKey, override val label: String) : ExtraKey
}

@Immutable
data class KeyModifiers(val ctrl: Boolean = false, val alt: Boolean = false) {
    val none: Boolean get() = !ctrl && !alt
}

/** A key press produced by the row: the key plus the modifiers that were active for it. */
@Immutable
data class ExtraKeyEvent(val key: ExtraKey, val modifiers: KeyModifiers)

/**
 * Modifier latch state, hoisted so that text typed on the IME also honours a latched Ctrl/Alt:
 * the terminal / editor calls [consume] for every IME key and applies the returned modifiers.
 */
@Stable
class ExtraKeysState {
    var ctrl by mutableStateOf(Latch.Off)
        private set
    var alt by mutableStateOf(Latch.Off)
        private set
    private var lastTapKey: ModifierKey? = null
    private var lastTapTime = 0L

    fun latch(key: ModifierKey): Latch = if (key == ModifierKey.Ctrl) ctrl else alt

    /** Tap on a modifier key at [uptimeMillis]. Two taps within 300ms lock it. */
    fun tap(key: ModifierKey, uptimeMillis: Long) {
        val current = latch(key)
        val double = lastTapKey == key && uptimeMillis - lastTapTime < DOUBLE_TAP_MILLIS
        val next = when {
            current == Latch.Locked -> Latch.Off
            current == Latch.OneShot && double -> Latch.Locked
            current == Latch.OneShot -> Latch.Off
            else -> Latch.OneShot
        }
        set(key, next)
        lastTapKey = if (next == Latch.OneShot) key else null
        lastTapTime = uptimeMillis
    }

    /** Returns the active modifiers and releases one-shot latches. */
    fun consume(): KeyModifiers {
        val result = KeyModifiers(ctrl = ctrl != Latch.Off, alt = alt != Latch.Off)
        if (ctrl == Latch.OneShot) ctrl = Latch.Off
        if (alt == Latch.OneShot) alt = Latch.Off
        return result
    }

    fun reset() { ctrl = Latch.Off; alt = Latch.Off }

    private fun set(key: ModifierKey, value: Latch) { if (key == ModifierKey.Ctrl) ctrl = value else alt = value }

    companion object { const val DOUBLE_TAP_MILLIS = 300L }
}

/** Key layouts. Groups are separated by 8dp. */
object ExtraKeyLayouts {
    private fun sp(key: SpecialKey, label: String, repeat: Boolean = false) = ExtraKey.Special(key, label, repeat)

    /** Terminal: `Esc Tab Ctrl Alt │ ← ↓ ↑ → │ Home End PgUp PgDn │ | ~ / - _ │ Del ^C ^D ^Z │ F1…F12`. */
    val Terminal: List<List<ExtraKey>> = listOf(
        listOf(sp(SpecialKey.Escape, "Esc"), sp(SpecialKey.Tab, "Tab"), ExtraKey.Latching(ModifierKey.Ctrl, "Ctrl"), ExtraKey.Latching(ModifierKey.Alt, "Alt")),
        listOf(sp(SpecialKey.Left, "←", true), sp(SpecialKey.Down, "↓", true), sp(SpecialKey.Up, "↑", true), sp(SpecialKey.Right, "→", true)),
        listOf(sp(SpecialKey.Home, "Home"), sp(SpecialKey.End, "End"), sp(SpecialKey.PageUp, "PgUp", true), sp(SpecialKey.PageDown, "PgDn", true)),
        listOf("|", "~", "/", "-", "_").map { ExtraKey.Text(it) },
        listOf(sp(SpecialKey.Delete, "Del", true), ExtraKey.Control('C'), ExtraKey.Control('D'), ExtraKey.Control('Z')),
        SpecialKey.entries.filter { it.name.startsWith("F") }.map { sp(it, it.name) },
    )

    /** Editor (docs/ui.md §8-4): Tab, arrows and common brackets. */
    val Editor: List<List<ExtraKey>> = listOf(
        listOf(sp(SpecialKey.Tab, "Tab")),
        listOf(sp(SpecialKey.Left, "←", true), sp(SpecialKey.Down, "↓", true), sp(SpecialKey.Up, "↑", true), sp(SpecialKey.Right, "→", true)),
        listOf(sp(SpecialKey.Home, "Home"), sp(SpecialKey.End, "End")),
        listOf("(", ")", "[", "]", "{", "}", "<", ">", "\"", "'", "`", "=", ";", ":").map { ExtraKey.Text(it) },
    )
}

/**
 * The special-key row above the IME, shared by terminal and editor (docs/ui.md §4.4). Scrolls
 * horizontally; keys are `surfaceContainerHigh`, `xs` corners, min 40dp wide. Ctrl/Alt latch through
 * [state]; arrows (and other `repeat` keys) auto-repeat while held. Emits KEYBOARD_TAP haptics.
 * Keys never take focus, so the IME stays attached to the terminal/editor.
 */
@Composable
fun ExtraKeysRow(
    state: ExtraKeysState,
    onKey: (ExtraKeyEvent) -> Unit,
    modifier: Modifier = Modifier,
    groups: List<List<ExtraKey>> = ExtraKeyLayouts.Terminal,
) {
    val dimens = WorkflowTheme.dimens
    Row(
        modifier
            .fillMaxWidth()
            .heightIn(min = dimens.extraKeysRow)
            .background(WorkflowTheme.colors.surfaceContainer)
            .horizontalScroll(rememberScrollState())
            .padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        groups.forEachIndexed { index, group ->
            if (index > 0) Spacer(Modifier.width(4.dp)) // 4 + 4 spacing = 8dp between groups
            group.forEach { key -> ExtraKeyButton(key, state, onKey) }
        }
    }
}

@Composable
private fun ExtraKeyButton(key: ExtraKey, state: ExtraKeysState, onKey: (ExtraKeyEvent) -> Unit) {
    val dimens = WorkflowTheme.dimens
    val colors = WorkflowTheme.colors
    val view = LocalView.current
    val currentOnKey by rememberUpdatedState(onKey)
    var pressed by remember { mutableStateOf(false) }
    val latch = (key as? ExtraKey.Latching)?.let { state.latch(it.modifier) } ?: Latch.Off
    val container = when {
        latch == Latch.Locked -> colors.primary
        latch == Latch.OneShot -> colors.secondaryContainer
        pressed -> colors.surfaceContainerHighest
        else -> colors.surfaceContainerHigh
    }
    val content = when (latch) {
        Latch.Locked -> colors.onPrimary
        Latch.OneShot -> colors.onSecondaryContainer
        Latch.Off -> colors.onSurface
    }
    fun emit() {
        view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
        currentOnKey(ExtraKeyEvent(key, state.consume()))
    }
    Box(
        Modifier
            .height(dimens.extraKey)
            .widthIn(min = 40.dp)
            .clip(WorkflowShapes.xs)
            .background(container)
            .drawBehind {
                if (latch == Latch.Locked) {
                    val y = size.height - 5.dp.toPx()
                    drawLine(content, Offset(size.width * 0.3f, y), Offset(size.width * 0.7f, y), strokeWidth = 1.5.dp.toPx())
                }
            }
            .pointerInput(key) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    down.consume()
                    pressed = true
                    try {
                        when (key) {
                            is ExtraKey.Latching -> {
                                val up = waitForUpOrCancellation()
                                if (up != null) {
                                    view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                                    state.tap(key.modifier, up.uptimeMillis)
                                }
                            }
                            is ExtraKey.Special -> if (key.repeat) {
                                emit()
                                // Auto-repeat: first after 400ms, then every 50ms until released.
                                var released = withTimeoutOrNull(400) { waitForUpOrCancellation(); true } ?: false
                                while (!released) {
                                    emit()
                                    released = withTimeoutOrNull(50) { waitForUpOrCancellation(); true } ?: false
                                }
                            } else if (waitForUpOrCancellation() != null) emit()
                            else -> if (waitForUpOrCancellation() != null) emit()
                        }
                    } finally {
                        pressed = false
                    }
                }
            }
            .semantics {
                role = Role.Button
                contentDescription = key.description
                if (key is ExtraKey.Latching) stateDescription = when (latch) { Latch.Off -> "关"; Latch.OneShot -> "单次"; Latch.Locked -> "锁定" }
            }
            .padding(horizontal = 8.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(key.label, style = WorkflowTheme.text.mono, color = content, maxLines = 1)
    }
}
