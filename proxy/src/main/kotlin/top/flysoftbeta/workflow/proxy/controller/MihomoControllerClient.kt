package top.flysoftbeta.workflow.proxy.controller

import java.io.IOException
import java.net.InetSocketAddress
import java.net.Proxy
import java.net.Socket
import java.net.URI
import java.net.URLEncoder
import java.util.concurrent.TimeUnit
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.channels.trySendBlocking
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import top.flysoftbeta.workflow.proxy.config.ProxyControllerSettings
import top.flysoftbeta.workflow.proxy.io.readProxyBytes
import top.flysoftbeta.workflow.proxy.redact.redactSecret

/** A controller HTTP failure. [message] is fixed text plus the controller's own error message, secret redacted. */
class ControllerException(val status: Int, message: String) : IOException(message)

/** Bounded controller response. [body] may contain anything the kernel returns: never log it raw. */
class ControllerResponse(val status: Int, val body: String)

/**
 * Loopback Mihomo REST transport. The secret is sent only as a bearer header to a loopback endpoint,
 * never through a system proxy and never across a redirect.
 */
class MihomoControllerClient(private val http: OkHttpClient = defaultHttpClient()) {
    private val streamingHttp: OkHttpClient by lazy {
        http.newBuilder().readTimeout(0, TimeUnit.MILLISECONDS).callTimeout(0, TimeUnit.MILLISECONDS).build()
    }

    /** Blocking request kept for callers already on an IO thread. Throws [ControllerException] on non-2xx. */
    fun request(controller: ProxyControllerSettings, method: String, path: String, body: String? = null): String {
        val response = http.newCall(build(controller, method, path, body)).execute().use { read(it) }
        return checked(controller, response).body
    }

    /** Cancellable request. Returns non-2xx responses too; callers decide, e.g. delay tests map 408/503/504. */
    suspend fun call(controller: ProxyControllerSettings, method: String, path: String, body: String? = null, timeoutMs: Long? = null): ControllerResponse {
        val client = if (timeoutMs == null) http else http.newBuilder()
            .readTimeout(timeoutMs + 2000, TimeUnit.MILLISECONDS).callTimeout(timeoutMs + 3000, TimeUnit.MILLISECONDS).build()
        val call = client.newCall(build(controller, method, path, body))
        return suspendCancellableCoroutine { continuation ->
            continuation.invokeOnCancellation { call.cancel() }
            call.enqueue(object : Callback {
                override fun onFailure(call: Call, e: IOException) {
                    // OkHttp messages carry host/port only; never headers.
                    continuation.resumeWithException(IOException("代理控制接口不可达", e))
                }
                override fun onResponse(call: Call, response: Response) {
                    val result = runCatching { response.use { read(it) } }
                    result.fold({ continuation.resume(it) }, { continuation.resumeWithException(IOException("代理控制接口响应读取失败", it)) })
                }
            })
        }
    }

    suspend fun callChecked(controller: ProxyControllerSettings, method: String, path: String, body: String? = null): String =
        checked(controller, call(controller, method, path, body)).body

    /**
     * Newline-delimited JSON stream (`/traffic`, `/logs`). Each line is bounded; cancelling the collector
     * cancels the HTTP call. The flow completes when the kernel closes the stream.
     */
    fun lines(controller: ProxyControllerSettings, path: String): Flow<String> = callbackFlow {
        val call = streamingHttp.newCall(build(controller, "GET", path, null))
        val reader = Thread({
            try {
                call.execute().use { response ->
                    if (!response.isSuccessful) {
                        val text = runCatching { read(response) }.getOrNull()
                        close(errorFor(controller, response.code, text?.body)); return@use
                    }
                    val source = response.body.source()
                    while (!isClosedForSend) {
                        if (source.exhausted()) break
                        val line = source.readUtf8LineStrict(MAX_STREAM_LINE_BYTES.toLong())
                        if (line.isNotBlank()) trySendBlocking(line)
                    }
                    close()
                }
            } catch (error: IOException) {
                close(if (call.isCanceled()) null else IOException("代理控制接口数据流中断", error))
            } catch (error: RuntimeException) {
                close(IOException("代理控制接口数据流中断", error))
            }
        }, "workflow-proxy-stream").apply { isDaemon = true }
        reader.start()
        awaitClose { call.cancel() }
    }.flowOn(Dispatchers.IO)

    private fun build(controller: ProxyControllerSettings, method: String, path: String, body: String?): Request {
        require(path.startsWith("/")) { "控制请求路径无效" }
        val builder = Request.Builder().url(controller.endpoint + path).header("Accept", "application/json")
        if (controller.secret.isNotEmpty()) builder.header("Authorization", "Bearer ${controller.secret}")
        when (method) {
            "GET" -> builder.get()
            "PATCH" -> builder.patch(checkNotNull(body).toRequestBody(JSON_MEDIA))
            "PUT" -> builder.put((body ?: "").toRequestBody(JSON_MEDIA))
            else -> error("不支持的控制请求")
        }
        return builder.build()
    }

    private fun read(response: Response): ControllerResponse {
        val bytes = response.body.byteStream().use { readProxyBytes(it, MAX_RESPONSE_BYTES) }
        return ControllerResponse(response.code, bytes.toString(Charsets.UTF_8))
    }

    private fun checked(controller: ProxyControllerSettings, response: ControllerResponse): ControllerResponse {
        if (response.status in 200..299) return response
        throw errorFor(controller, response.status, response.body)
    }

    companion object {
        const val MAX_RESPONSE_BYTES = 8 * 1024 * 1024
        const val MAX_STREAM_LINE_BYTES = 256 * 1024
        private val JSON_MEDIA = "application/json; charset=utf-8".toMediaType()

        /** Never routed through a system proxy and never redirected: the secret stays on loopback. */
        fun defaultHttpClient(): OkHttpClient = OkHttpClient.Builder().proxy(Proxy.NO_PROXY).followRedirects(false).followSslRedirects(false)
            .retryOnConnectionFailure(false)
            .connectTimeout(3, TimeUnit.SECONDS).readTimeout(8, TimeUnit.SECONDS).callTimeout(10, TimeUnit.SECONDS).build()

        fun proxyPath(group: String): String = "/proxies/${encode(group)}"

        fun encode(value: String): String = URLEncoder.encode(value, "UTF-8").replace("+", "%20")

        /** Controller error `{"message": …}` → exception with a fixed prefix and the redacted kernel message. */
        internal fun errorFor(controller: ProxyControllerSettings, status: Int, body: String?): ControllerException {
            val kernelMessage = body?.let { ControllerJson.errorMessage(it) }?.let { redactSecret(it, controller.secret) }?.take(300)
            val prefix = when (status) {
                401 -> "控制接口拒绝了配置中的 secret"
                404 -> "控制接口找不到该对象"
                else -> "代理控制请求失败（HTTP $status）"
            }
            return ControllerException(status, if (kernelMessage.isNullOrBlank() || status == 401) prefix else "$prefix：$kernelMessage")
        }

        /** True if something already listens on the controller endpoint (checked before launching a kernel). */
        fun acceptsConnections(endpoint: String): Boolean = try {
            val uri = URI(endpoint)
            Socket().use { it.connect(InetSocketAddress(uri.host.removePrefix("[").removeSuffix("]"), uri.port), 300) }
            true
        } catch (_: IOException) { false }
    }
}
