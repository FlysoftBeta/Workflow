package top.flysoftbeta.workflow.feature.chat.transcript

/** Explicit native UI actions; resource reads are suspendable and all other callbacks run on main. */
interface TranscriptCallbacks {
    fun openUrl(url: String)
    fun openPath(path: String, line: Int?, column: Int?)
    fun copyText(text: String)
    fun copyTurn(turn: String, what: String)
    fun toggle(turn: String, expanded: Boolean)
    fun earlier()
    fun layout(atBottom: Boolean)
    fun action(turn: String, name: String)
    fun viewport(anchor: String?, offset: Int, atBottom: Boolean) {}
    suspend fun resource(path: String, maxBytes: Int): ByteArray? = null
}
