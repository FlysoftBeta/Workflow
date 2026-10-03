package top.flysoftbeta.workflow.proxy

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.io.Closeable
import java.io.IOException
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.URLDecoder
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicInteger
import top.flysoftbeta.workflow.proxy.config.ProxyControllerSettings

/**
 * A loopback HTTP server speaking the Mihomo 1.19 controller subset used by the app (payload shapes copied from
 * hub/route and adapter MarshalJSON in the vendored v1.19.31 source).
 */
class FakeMihomo(val secret: String = "s3cr3t-value", port: Int = 0) : Closeable {
    data class Call(val method: String, val path: String, val query: String?, val authorization: String?, val body: String)

    val calls = CopyOnWriteArrayList<Call>()
    @Volatile var mode = "rule"
    val selected = ConcurrentHashMap(mapOf("Proxy" to "HK 01", "Auto" to "HK 01", "GLOBAL" to "DIRECT"))
    val delays = ConcurrentHashMap(mapOf("HK 01" to 120, "JP 02" to 0, "DIRECT" to 3))
    @Volatile var ignoreSelection = false
    @Volatile var errorBody: String? = null
    val openStreams = AtomicInteger()
    val trafficLines = AtomicInteger()
    val providerUpdates = CopyOnWriteArrayList<String>()
    var healthcheckStatus = 204
    private val server = HttpServer.create(InetSocketAddress(InetAddress.getByName("127.0.0.1"), port), 16)
    val port: Int get() = server.address.port
    val settings: ProxyControllerSettings get() = ProxyControllerSettings("http://127.0.0.1:$port", secret)

    init {
        server.executor = Executors.newCachedThreadPool { Thread(it, "fake-mihomo").apply { isDaemon = true } }
        server.createContext("/") { exchange -> try { handle(exchange) } catch (_: IOException) { } finally { exchange.close() } }
        server.start()
    }

    private fun proxiesJson(): String {
        fun node(name: String, type: String) = """"$name":{"name":"$name","type":"$type","alive":${(delays[name] ?: 0) > 0},"udp":true,"history":[{"time":"2026-09-27T00:00:00Z","delay":${delays[name] ?: 0}}],"extra":{}}"""
        fun group(name: String, type: String, all: List<String>, extra: String = "") =
            """"$name":{"name":"$name","type":"$type","now":"${selected[name] ?: ""}","all":[${all.joinToString(",") { "\"$it\"" }}],"hidden":false,"icon":"","history":[],"alive":true,"udp":true$extra}"""
        return "{\"proxies\":{" + listOf(
            node("DIRECT", "Direct"), node("REJECT", "Reject"), node("HK 01", "Vmess"), node("JP 02", "Trojan"),
            group("Auto", "URLTest", listOf("HK 01", "JP 02"), ",\"testUrl\":\"https://cp.example/generate_204\",\"fixed\":\"\""),
            group("Proxy", "Selector", listOf("Auto", "HK 01", "JP 02", "DIRECT")),
            group("Hidden", "Selector", listOf("DIRECT")).replace("\"hidden\":false", "\"hidden\":true"),
            // GLOBAL.all is configuration order: Proxy before Auto on purpose.
            group("GLOBAL", "Selector", listOf("DIRECT", "REJECT", "Proxy", "Auto", "Hidden", "HK 01", "JP 02")),
        ).joinToString(",") + "}}"
    }

    private fun handle(exchange: HttpExchange) {
        val method = exchange.requestMethod
        val rawPath = exchange.requestURI.rawPath
        val segments = rawPath.split('/').filter { it.isNotEmpty() }.map { URLDecoder.decode(it.replace("+", "%2B"), "UTF-8") }
        val body = exchange.requestBody.readBytes().toString(Charsets.UTF_8)
        calls += Call(method, rawPath, exchange.requestURI.rawQuery, exchange.requestHeaders.getFirst("Authorization"), body)
        if (exchange.requestHeaders.getFirst("Authorization") != "Bearer $secret") return send(exchange, 401, """{"message":"Unauthorized"}""")
        errorBody?.let { return send(exchange, 500, it) }
        when {
            segments == listOf("version") -> send(exchange, 200, """{"meta":true,"version":"v1.19.31"}""")
            segments == listOf("configs") && method == "GET" -> send(exchange, 200, """{"port":0,"mixed-port":17890,"mode":"$mode","log-level":"info"}""")
            segments == listOf("configs") && method == "PATCH" -> {
                Regex("\"mode\":\"(\\w+)\"").find(body)?.let { mode = it.groupValues[1] }
                exchange.sendResponseHeaders(204, -1)
            }
            segments == listOf("proxies") -> send(exchange, 200, proxiesJson())
            segments.size == 2 && segments[0] == "proxies" && method == "PUT" -> {
                val name = Regex("\"name\":\"([^\"]+)\"").find(body)?.groupValues?.get(1)
                if (!ignoreSelection && name != null) selected[segments[1]] = name
                exchange.sendResponseHeaders(204, -1)
            }
            segments.size == 2 && segments[0] == "proxies" -> send(exchange, 200, """{"name":"${segments[1]}","now":"${selected[segments[1]] ?: ""}"}""")
            segments.size == 3 && segments[0] == "proxies" && segments[2] == "delay" -> {
                val delay = delays[segments[1]]
                when {
                    delay == null -> send(exchange, 404, """{"message":"Resource not found"}""")
                    delay < 0 -> send(exchange, 408, """{"message":"Timeout"}""")
                    delay == 0 -> send(exchange, 503, """{"message":"An error occurred in the delay test"}""")
                    else -> send(exchange, 200, """{"delay":$delay}""")
                }
            }
            segments.size == 3 && segments[0] == "group" && segments[2] == "delay" -> {
                val members = mapOf("Auto" to listOf("HK 01", "JP 02"), "Proxy" to listOf("Auto", "HK 01", "JP 02", "DIRECT"))[segments[1]]
                    ?: return send(exchange, 404, """{"message":"Resource not found"}""")
                val ok = members.mapNotNull { name -> delays[name]?.takeIf { it > 0 }?.let { "\"$name\":$it" } }
                if (ok.isEmpty()) send(exchange, 504, """{"message":"get delay: all proxies timeout"}""")
                else send(exchange, 200, "{" + ok.joinToString(",") + "}")
            }
            segments == listOf("traffic") -> stream(exchange) { index -> """{"up":${index * 10},"down":${index * 100},"upTotal":${index * 1000},"downTotal":${index * 10000}}""" }
            segments == listOf("logs") -> stream(exchange) { index -> """{"type":"info","payload":"line $index auth=$secret"}""" }
            segments == listOf("connections") -> send(exchange, 200, """{"downloadTotal":10,"uploadTotal":20,"connections":[{"id":"c1","metadata":{"network":"tcp","type":"Tun","sourceIP":"198.18.0.1","destinationIP":"93.184.216.34","sourcePort":"40000","destinationPort":"443","host":"example.com","process":"","uid":10100},"upload":1,"download":2,"start":"2026-09-27T00:00:00Z","chains":["HK 01","Proxy"],"rule":"Match","rulePayload":""}],"memory":0}""")
            segments == listOf("providers", "proxies") -> send(exchange, 200, """{"providers":{"default":{"name":"default","type":"Proxy","vehicleType":"Compatible","proxies":[]},"sub":{"name":"sub","type":"Proxy","vehicleType":"HTTP","proxies":[{"name":"HK 01"}],"updatedAt":"2026-09-27T00:00:00Z"},"bad":{"name":"bad","type":"Proxy","vehicleType":"HTTP","proxies":[]}}}""")
            segments == listOf("providers", "rules") -> send(exchange, 200, """{"providers":{"ads":{"name":"ads","type":"Rule","vehicleType":"HTTP","behavior":"Domain","format":"YamlRule","ruleCount":42,"updatedAt":"2026-09-27T00:00:00Z"}}}""")
            segments.size == 3 && segments[0] == "providers" && method == "PUT" -> {
                providerUpdates += "${segments[1]}/${segments[2]}"
                if (segments[2] == "bad") send(exchange, 503, """{"message":"fetch failed token=$secret"}""") else exchange.sendResponseHeaders(204, -1)
            }
            segments.size == 4 && segments[3] == "healthcheck" -> send(exchange, healthcheckStatus, if (healthcheckStatus == 204) "" else """{"message":"healthcheck failed $secret"}""")
            else -> send(exchange, 404, """{"message":"Resource not found"}""")
        }
    }

    private fun stream(exchange: HttpExchange, line: (Int) -> String) {
        exchange.responseHeaders.add("Content-Type", "application/json")
        exchange.sendResponseHeaders(200, 0)
        openStreams.incrementAndGet()
        try {
            var index = 1
            while (true) {
                exchange.responseBody.write((line(index++) + "\n").toByteArray())
                exchange.responseBody.flush()
                if (exchange.requestURI.rawPath == "/traffic") trafficLines.incrementAndGet()
                Thread.sleep(40)
            }
        } catch (_: IOException) {
        } catch (_: InterruptedException) {
        } finally { openStreams.decrementAndGet() }
    }

    private fun send(exchange: HttpExchange, status: Int, body: String) {
        val bytes = body.toByteArray()
        exchange.responseHeaders.add("Content-Type", "application/json")
        exchange.sendResponseHeaders(status, bytes.size.toLong())
        exchange.responseBody.write(bytes)
    }

    override fun close() { server.stop(0); (server.executor as java.util.concurrent.ExecutorService).shutdownNow() }
}
