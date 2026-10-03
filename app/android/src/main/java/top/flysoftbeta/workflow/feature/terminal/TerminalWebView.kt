package top.flysoftbeta.workflow.feature.terminal

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Color
import android.util.Log
import android.view.ViewGroup
import android.webkit.ConsoleMessage
import android.webkit.JavascriptInterface
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.webkit.WebViewAssetLoader
import java.io.ByteArrayInputStream
import org.json.JSONArray
import org.json.JSONObject

/** What the terminal page reports to its controller (always delivered on the main thread). */
internal interface TerminalPageListener {
    fun onReady()
    fun onInput(data: String)
    /** Extra-keys data chosen by the page (cursor mode applied); written as is. */
    fun onKey(data: String)
    fun onResize(columns: Int, rows: Int)
    fun onOpenUrl(url: String)
    fun onOpenPath(text: String)
    fun onCheckLinks(requestId: Int, texts: List<String>)
    fun onSelection(text: String, x: Int, y: Int)
    fun onGone()
}

/**
 * The offline xterm page (assets/web/terminal.html). Kept by its panel controller across compositions;
 * created with a context wrapper, never an Activity. Only app assets load; the bridge is removed on
 * [release]. Calls before the page is ready are queued.
 */
@SuppressLint("SetJavaScriptEnabled", "ViewConstructor")
internal class TerminalWebView(context: Context, private val listener: TerminalPageListener) : WebView(context) {
    private var ready = false
    private var released = false
    private val queued = ArrayList<String>()

    init {
        val loader = WebViewAssetLoader.Builder().addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(context)).build()
        setBackgroundColor(Color.TRANSPARENT)
        // WRAP_CONTENT gives WebView a zero layout viewport on Android 9 even when Compose bounds it.
        layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        isVerticalScrollBarEnabled = false
        isHorizontalScrollBarEnabled = false
        overScrollMode = OVER_SCROLL_NEVER
        // The page does its own long-press selection (with the app's 复制 · 粘贴 · 全选 bar): no native
        // text-selection action mode on xterm's hidden textarea.
        isLongClickable = false
        setOnLongClickListener { true }
        isHapticFeedbackEnabled = false
        settings.javaScriptEnabled = true
        settings.allowFileAccess = false
        settings.allowContentAccess = false
        settings.domStorageEnabled = false
        settings.setSupportMultipleWindows(false)
        settings.setSupportZoom(false)
        settings.textZoom = 100
        settings.mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
        addJavascriptInterface(Bridge(), "Workflow")
        webChromeClient = object : WebChromeClient() {
            override fun onConsoleMessage(message: ConsoleMessage): Boolean {
                if (message.messageLevel() == ConsoleMessage.MessageLevel.ERROR)
                    Log.w("WorkflowTerminal", "page error at ${message.lineNumber()}: ${message.message().take(300)}")
                return true
            }
        }
        webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse =
                loader.shouldInterceptRequest(request.url)
                    ?: WebResourceResponse("text/plain", "UTF-8", 403, "Blocked", emptyMap(), ByteArrayInputStream(byteArrayOf()))

            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean = true

            override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
                (parent as? ViewGroup)?.removeView(view)
                release()
                listener.onGone()
                return true
            }
        }
        loadUrl("https://appassets.androidplatform.net/assets/web/terminal.html")
    }

    val isReady: Boolean get() = ready && !released

    /** Runs `WorkflowTerminal.<call>` now, or once the page is ready. */
    fun call(script: String) {
        if (released) return
        if (!ready) { queued += script; return }
        evaluateJavascript("WorkflowTerminal.$script", null)
    }

    fun write(text: String) { if (text.isNotEmpty()) call("write(${JSONObject.quote(text)})") }
    fun reset(text: String) = call("reset(${JSONObject.quote(text)})")
    fun key(normal: String, application: String?) =
        call("key(${JSONObject.quote(normal)}, ${application?.let(JSONObject::quote) ?: "null"})")
    fun paste(text: String) = call("paste(${JSONObject.quote(text)})")
    fun theme(theme: JSONObject, link: String) = call("theme($theme, ${JSONObject.quote(link)})")
    fun font(size: Float) = call("font($size)")
    fun linksChecked(id: Int, results: List<Boolean>) = call("linksChecked($id, ${JSONArray(results)})")
    fun invalidateLinks() = call("invalidateLinks()")
    fun focusTerminal() = call("focus()")
    fun selectAll() = call("selectAll()")
    fun clearSelection() = call("clearSelection()")
    fun clearScreen() = call("clear()")

    override fun onSizeChanged(width: Int, height: Int, oldWidth: Int, oldHeight: Int) {
        super.onSizeChanged(width, height, oldWidth, oldHeight)
        if (ready) post { call("fit()") }
    }

    fun release() {
        if (released) return
        released = true
        ready = false
        queued.clear()
        stopLoading()
        removeJavascriptInterface("Workflow")
        (parent as? ViewGroup)?.removeView(this)
        destroy()
    }

    private inner class Bridge {
        @JavascriptInterface fun ready() = post {
            if (released) return@post
            ready = true
            val pending = queued.toList()
            queued.clear()
            listener.onReady()
            pending.forEach { evaluateJavascript("WorkflowTerminal.$it", null) }
        }
        @JavascriptInterface fun input(data: String) { post { if (!released) listener.onInput(data) } }
        @JavascriptInterface fun key(data: String) { post { if (!released) listener.onKey(data) } }
        @JavascriptInterface fun resize(columns: Int, rows: Int) { post { if (!released) listener.onResize(columns.coerceIn(2, 500), rows.coerceIn(1, 300)) } }
        @JavascriptInterface fun openUrl(url: String) { post { if (!released) listener.onOpenUrl(url) } }
        @JavascriptInterface fun openPath(text: String) { post { if (!released) listener.onOpenPath(text) } }
        @JavascriptInterface fun checkLinks(id: Int, json: String) {
            val texts = runCatching { JSONArray(json).let { array -> List(array.length()) { array.getString(it) } } }.getOrDefault(emptyList())
            post { if (!released) listener.onCheckLinks(id, texts.take(128)) }
        }
        @JavascriptInterface fun selection(text: String, x: Int, y: Int) { post { if (!released) listener.onSelection(text, x, y) } }
    }
}
