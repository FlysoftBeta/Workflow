package top.flysoftbeta.workflow.feature.terminal

import android.content.ClipData
import android.content.ClipboardManager
import android.content.res.Resources
import android.graphics.Canvas
import android.graphics.Rect
import android.graphics.RectF
import android.graphics.drawable.Drawable
import android.view.ActionMode
import android.view.Menu
import android.view.MenuItem
import android.view.MotionEvent
import android.view.View
import java.util.Locale
import kotlin.math.max
import kotlin.math.roundToInt
import org.json.JSONObject

/**
 * Platform text-selection chrome over the terminal page: the theme's teardrop handles drawn on the WebView and
 * the floating text-selection toolbar ([ActionMode.TYPE_FLOATING]). The selection itself stays in xterm's buffer:
 * the page reports geometry through `selectionState(json)` and the settled copy text through `selection(text, x, y)`.
 * Handle drags are consumed natively, before Chromium sees them, and forwarded to `WorkflowTerminal.dragHandle`.
 */
internal class TerminalSelectionChrome(private val view: TerminalWebView) {
    private class Endpoint(val x: Float, val y: Float, val visible: Boolean)
    private class State(val toolbar: Boolean, val start: Endpoint, val end: Endpoint, val rect: RectF?, val width: Float)
    private class Drag(val edge: Int, val grabX: Float, val grabY: Float)

    private var state: State? = null
    private var drag: Drag? = null
    private var mode: ActionMode? = null
    private var finishingMode = false
    private var handleTheme: Resources.Theme? = null
    private val handles = arrayOfNulls<Drawable>(2)
    private var tint: Int? = null
    /** The settled selection text last reported by the page; Copy writes exactly this text. */
    var text: String = ""
        private set

    /** The live floating toolbar, if shown (instrumentation inspects it). */
    val actionMode: ActionMode? get() = mode

    fun onSelectionText(value: String) {
        if (value == text) return
        text = value
        mode?.invalidate()
    }

    fun update(json: String) {
        val o = runCatching { JSONObject(json) }.getOrNull() ?: return
        state = if (!o.optBoolean("active")) null else State(
            toolbar = o.optBoolean("toolbar"),
            start = endpoint(o.optJSONObject("start")),
            end = endpoint(o.optJSONObject("end")),
            rect = o.optJSONObject("rect")?.let {
                RectF(it.optDouble("left").toFloat(), it.optDouble("top").toFloat(), it.optDouble("right").toFloat(), it.optDouble("bottom").toFloat())
            },
            width = o.optDouble("width", 0.0).toFloat(),
        )
        view.invalidate()
        syncToolbar()
    }

    fun setTint(color: Int?) {
        tint = color
        handles.forEach { drawable -> color?.let { drawable?.setTint(it) } }
        view.invalidate()
    }

    /** Visible handle drawables in view coordinates. */
    fun handleBounds(): List<Rect> = listOf(START, END).mapNotNull(::bounds)

    fun draw(canvas: Canvas) {
        if (state == null) return
        canvas.save()
        canvas.translate(view.scrollX.toFloat(), view.scrollY.toFloat())
        for (edge in listOf(START, END)) {
            val (drawable, bounds) = placement(edge) ?: continue
            drawable.bounds = bounds
            drawable.draw(canvas)
        }
        canvas.restore()
    }

    /** Consumes a whole gesture that starts on a handle; any other gesture is left to the page. */
    fun onTouchEvent(event: MotionEvent): Boolean {
        val current = drag
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val edge = hit(event.x, event.y) ?: return false
                val anchor = anchor(edge) ?: return false
                drag = Drag(edge, event.x - anchor.first, event.y - anchor.second)
                send("start", event)
                syncToolbar()
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                current ?: return false
                send("move", event)
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                current ?: return false
                send(if (event.actionMasked == MotionEvent.ACTION_UP) "end" else "cancel", event)
                drag = null
                syncToolbar()
                return true
            }
            else -> return current != null
        }
    }

    fun attached() = syncToolbar()

    fun detached() = finishMode()

    fun release() {
        finishMode()
        state = null
        drag = null
    }

    private fun endpoint(o: JSONObject?) = Endpoint(
        o?.optDouble("x", 0.0)?.toFloat() ?: 0f, o?.optDouble("y", 0.0)?.toFloat() ?: 0f, o?.optBoolean("visible") == true,
    )

    /** View pixels per CSS pixel of the page's layout viewport. */
    private fun scale(): Float {
        val width = state?.width ?: 0f
        return if (width > 0f && view.width > 0) view.width / width else view.resources.displayMetrics.density
    }

    private fun handle(edge: Int): Drawable? {
        val theme = view.context.theme
        if (handleTheme !== theme) {
            handleTheme = theme
            handles[START] = load(theme, android.R.attr.textSelectHandleLeft)
            handles[END] = load(theme, android.R.attr.textSelectHandleRight)
            tint?.let { color -> handles.forEach { it?.setTint(color) } }
        }
        return handles[edge]
    }

    private fun load(theme: Resources.Theme, attr: Int): Drawable? {
        val attributes = theme.obtainStyledAttributes(intArrayOf(attr))
        return try { attributes.getDrawable(0)?.mutate() } finally { attributes.recycle() }
    }

    /** The selection endpoint the handle points at (a row's bottom edge), in view coordinates. */
    private fun anchor(edge: Int): Pair<Float, Float>? {
        val point = state?.let { if (edge == START) it.start else it.end }?.takeIf { it.visible } ?: return null
        val scale = scale()
        return point.x * scale to point.y * scale
    }

    private fun bounds(edge: Int): Rect? = placement(edge)?.second

    /**
     * As in the platform editor, the left-pointing teardrop's hotspot is three quarters across and the right one's a
     * quarter. A handle that the view edge would clip is mirrored, as Chromium does, so it stays whole and grabbable.
     */
    private fun placement(edge: Int): Pair<Drawable, Rect>? {
        val (x, y) = anchor(edge) ?: return null
        fun place(orientation: Int): Pair<Drawable, Rect>? {
            val drawable = handle(orientation) ?: return null
            val width = drawable.intrinsicWidth
            val left = x.roundToInt() - if (orientation == START) width * 3 / 4 else width / 4
            return drawable to Rect(left, y.roundToInt(), left + width, y.roundToInt() + drawable.intrinsicHeight)
        }
        val preferred = place(edge) ?: return null
        val clipped = if (edge == START) preferred.second.left < 0 else preferred.second.right > view.width
        return if (clipped) place(1 - edge) ?: preferred else preferred
    }

    private fun hit(x: Float, y: Float): Int? {
        val minimum = view.resources.displayMetrics.density * 48f
        return listOf(START, END).mapNotNull { edge ->
            val bounds = bounds(edge) ?: return@mapNotNull null
            val halfWidth = max(bounds.width().toFloat(), minimum) / 2f
            val height = max(bounds.height().toFloat(), minimum)
            val inside = x >= bounds.exactCenterX() - halfWidth && x <= bounds.exactCenterX() + halfWidth &&
                y >= bounds.top - minimum / 4f && y <= bounds.top + height
            if (inside) edge to (x - bounds.exactCenterX()).let { it * it } + (y - bounds.exactCenterY()).let { it * it } else null
        }.minByOrNull { it.second }?.first
    }

    private fun send(phase: String, event: MotionEvent) {
        val current = drag ?: return
        val scale = scale()
        val x = (event.x - current.grabX) / scale
        val y = (event.y - current.grabY) / scale
        val edge = if (current.edge == START) "start" else "end"
        view.call(String.format(Locale.ROOT, "dragHandle('%s', '%s', %.2f, %.2f)", edge, phase, x, y))
    }

    private fun syncToolbar() {
        val current = state
        val wanted = current != null && current.toolbar && drag == null && view.isAttachedToWindow && view.windowToken != null
        if (!wanted) { finishMode(); return }
        val existing = mode
        if (existing != null) existing.invalidateContentRect()
        else mode = view.startActionMode(callback, ActionMode.TYPE_FLOATING)
    }

    private fun finishMode() {
        val existing = mode ?: return
        finishingMode = true
        try { existing.finish() } finally { finishingMode = false }
        mode = null
    }

    private fun clipboard(): ClipboardManager? = view.context.getSystemService(ClipboardManager::class.java)

    private fun clipboardText(): String {
        val clip = clipboard()?.primaryClip?.takeIf { it.itemCount > 0 } ?: return ""
        return clip.getItemAt(0).coerceToText(view.context)?.toString().orEmpty()
    }

    private val callback = object : ActionMode.Callback2() {
        override fun onCreateActionMode(mode: ActionMode, menu: Menu): Boolean { populate(menu); return true }

        override fun onPrepareActionMode(mode: ActionMode, menu: Menu): Boolean { populate(menu); return true }

        override fun onActionItemClicked(mode: ActionMode, item: MenuItem): Boolean {
            when (item.itemId) {
                android.R.id.copy -> {
                    val copied = text
                    if (copied.isNotEmpty()) clipboard()?.setPrimaryClip(ClipData.newPlainText("终端", copied))
                    view.clearSelection()
                }
                android.R.id.paste -> {
                    val pasted = clipboardText()
                    view.clearSelection()
                    if (pasted.isNotEmpty()) view.paste(pasted)
                }
                android.R.id.selectAll -> view.selectAll()
                else -> return false
            }
            return true
        }

        override fun onDestroyActionMode(mode: ActionMode) {
            if (this@TerminalSelectionChrome.mode !== mode) return
            this@TerminalSelectionChrome.mode = null
            // Dismissed by the system (another selection, Back): the selection ends with its toolbar.
            if (!finishingMode) view.clearSelection()
        }

        override fun onGetContentRect(mode: ActionMode, view: View, outRect: Rect) {
            val current = state
            val rect = current?.rect
            if (current == null || rect == null) { outRect.set(-10_000, -10_000, -9_999, -9_999); return }
            val scale = scale()
            val below = bounds(END)?.height() ?: 0
            outRect.set((rect.left * scale).roundToInt(), (rect.top * scale).roundToInt(),
                (rect.right * scale).roundToInt(), (rect.bottom * scale).roundToInt() + below)
        }
    }

    private fun populate(menu: Menu) {
        menu.clear()
        if (text.isNotEmpty()) {
            menu.add(Menu.NONE, android.R.id.copy, 1, android.R.string.copy).setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
        }
        if (clipboard()?.hasPrimaryClip() == true) {
            menu.add(Menu.NONE, android.R.id.paste, 2, android.R.string.paste).setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
        }
        menu.add(Menu.NONE, android.R.id.selectAll, 3, android.R.string.selectAll).setShowAsAction(MenuItem.SHOW_AS_ACTION_IF_ROOM)
    }

    private companion object {
        const val START = 0
        const val END = 1
    }
}
