// 루담톡(LudamTalk) 온라인채팅 봇 SDK — Kotlin / Java (JVM 11 이상)
// 설명서: https://ludamtalk.com/developers
//
// Kotlin:
//   val bot = LudamBot(System.getenv("LUDAM_TOKEN"))
//   bot.onReady { me -> println("${me.name} 온라인!") }
//   bot.command("주사위", "주사위를 굴려요") { ctx -> ctx.reply("🎲 **${(1..6).random()}**") }
//   bot.run()
//
// Java:
//   LudamBot bot = new LudamBot(System.getenv("LUDAM_TOKEN"));
//   bot.commandJava("주사위", "주사위를 굴려요", ctx -> ctx.replyBlocking(Content.text("🎲 4")));
//   bot.run();
package com.ludamtalk.bot

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import okhttp3.Call
import okhttp3.Callback
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

const val LUDAM_API = "https://api.ludamtalk.com"
const val LUDAM_GATEWAY = "wss://api.ludamtalk.com/bot/gateway"
const val LUDAM_VERSION = "1.0.0"
private val CMD_RE = Regex("^[a-z0-9가-힣_]{1,20}$")

/** 루담 API 오류 (status: HTTP 번호, code: 오류 이름) */
class LudamException(val status: Int, val code: String, message: String? = null) : Exception(message ?: "루담 API 오류 ($status $code)")

/** 꾸민 카드 (5개까지, 칸 10개까지) */
class Embed {
    internal val json = JSONObject()
    fun title(t: String) = apply { json.put("title", t) }
    fun description(t: String) = apply { json.put("description", t) }
    fun url(u: String) = apply { json.put("url", u) }
    fun color(c: Int) = apply { json.put("color", c) }
    fun author(n: String) = apply { json.put("author", JSONObject().put("name", n)) }
    fun footer(t: String) = apply { json.put("footer", JSONObject().put("text", t)) }
    fun image(u: String) = apply { json.put("image", JSONObject().put("url", u)) }
    fun thumbnail(u: String) = apply { json.put("thumbnail", JSONObject().put("url", u)) }
    fun field(name: String, value: String, inline: Boolean = false) = apply {
        (json.optJSONArray("fields") ?: JSONArray().also { json.put("fields", it) }).put(JSONObject().put("name", name).put("value", value).put("inline", inline))
    }
}

/** 보낼 내용 (글자는 마크다운 가능). AI로 만든 답이면 ai = true → "✨ AI" 표시 */
data class Content(val text: String = "", val embeds: List<Embed> = emptyList(), val ai: Boolean = false) {
    companion object {
        @JvmStatic fun text(t: String) = Content(text = t)
        @JvmStatic fun embed(e: Embed) = Content(embeds = listOf(e))
    }
    fun withAi() = copy(ai = true)
    internal fun into(o: JSONObject): JSONObject = o.apply {
        if (text.isNotEmpty()) put("text", text)
        if (embeds.isNotEmpty()) put("embeds", JSONArray().apply { embeds.forEach { put(it.json) } })
        if (ai) put("ai", true)
    }
}

data class BotUser(val id: String, val name: String)
data class Room(val id: String, val title: String)
/** id: 방마다 다른 번호 (u+16자리) · name: 그 방 닉네임 · newAccount: 들어올 때만, 만든 지 7일 안 */
data class User(val id: String, val name: String, val newAccount: Boolean)
data class MessageInfo(val id: String, val text: String, val mentions: Int, val reply: Boolean)
data class Member(val id: String, val name: String, val role: String, val timeoutUntil: String?)

/** 봇 API (REST) */
class LudamApi internal constructor(private val token: String, private val http: OkHttpClient) {
    /** POST /bot/api/<path> */
    suspend fun call(path: String, body: JSONObject = JSONObject()): JSONObject = withContext(Dispatchers.IO) {
        val req = Request.Builder().url("$LUDAM_API/bot/api/$path")
            .header("Authorization", "Bot $token").header("User-Agent", "LudamBotKotlin/$LUDAM_VERSION")
            .post(body.toString().toRequestBody("application/json".toMediaType())).build()
        http.newCall(req).await().use { r ->
            val text = r.body?.string().orEmpty()
            val o = runCatching { JSONObject(text) }.getOrDefault(JSONObject())
            if (!r.isSuccessful) throw LudamException(r.code, o.optString("error", "error"), o.optString("error_description").ifEmpty { null })
            o
        }
    }
    suspend fun send(room: String, c: Content): String = call("send", c.into(JSONObject().put("room", room))).optString("id")
    suspend fun typing(room: String, ai: Boolean = false) { call("typing", JSONObject().put("room", room).put("ai", ai)) }
    // 🛡️ 보안 봇 (관리 권한)
    suspend fun members(room: String): Pair<List<Member>, Boolean> {
        val o = call("members", JSONObject().put("room", room))
        val a = o.optJSONArray("members") ?: JSONArray()
        return (0 until a.length()).map { i -> a.getJSONObject(i).let { Member(it.optString("id"), it.optString("name"), it.optString("role"), it.optString("timeoutUntil").ifEmpty { null }) } } to o.optBoolean("locked")
    }
    suspend fun timeout(room: String, user: String, minutes: Int = 10, reason: String = "") { call("timeout", JSONObject().put("room", room).put("user", user).put("minutes", minutes).put("reason", reason)) }
    suspend fun untimeout(room: String, user: String) { call("untimeout", JSONObject().put("room", room).put("user", user)) }
    suspend fun lock(room: String, locked: Boolean = true, reason: String = "") { call("lock", JSONObject().put("room", room).put("locked", locked).put("reason", reason)) }
    suspend fun purge(room: String, messageIds: List<String>): Int = call("purge", JSONObject().put("room", room).put("messages", JSONArray(messageIds.take(100)))).optInt("deleted")
    suspend fun unban(room: String, user: String) { call("unban", JSONObject().put("room", room).put("user", user)) }
    suspend fun rooms(): List<Room> {
        val out = ArrayList<Room>()
        var cursor = ""
        do {
            val o = call("rooms", if (cursor.isEmpty()) JSONObject() else JSONObject().put("cursor", cursor))
            val a = o.optJSONArray("rooms") ?: JSONArray()
            for (i in 0 until a.length()) a.getJSONObject(i).let { out.add(Room(it.optString("id"), it.optString("title"))) }
            cursor = o.optString("cursor")
        } while (cursor.isNotEmpty())
        return out
    }
}

private suspend fun Call.await(): Response = suspendCancellableCoroutine { k ->
    enqueue(object : Callback {
        override fun onFailure(call: Call, e: IOException) { if (k.isActive) k.resumeWithException(e) }
        override fun onResponse(call: Call, response: Response) { k.resume(response) }
    })
    k.invokeOnCancellation { runCatching { cancel() } }
}

/** 이벤트 하나 (ctx.reply / send / typing / delete / kick / timeout) */
class Context internal constructor(val api: LudamApi, val raw: JSONObject) {
    val type: String = raw.optString("type")
    val room = raw.optJSONObject("room").let { Room(it?.optString("id").orEmpty(), it?.optString("title").orEmpty()) }
    val user = raw.optJSONObject("user").let { User(it?.optString("id").orEmpty(), it?.optString("name").orEmpty(), it?.optBoolean("newAccount") == true) }
    val message = raw.optJSONObject("message").let { MessageInfo(it?.optString("id").orEmpty(), it?.optString("text").orEmpty(), it?.optInt("mentions") ?: 0, it?.optBoolean("reply") == true) }
    val text: String = raw.optString("text").ifEmpty { message.text }
    val command: String = raw.optString("command")
    val args: String = raw.optString("args")
    val argv: List<String> = args.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }
    val replyToken: String = raw.optString("replyToken")

    /** 이 메시지에 대답 (15분 안, 5개까지) */
    suspend fun reply(c: Content): String = api.call("reply", c.into(JSONObject().put("replyToken", replyToken))).optString("id")
    suspend fun reply(text: String): String = reply(Content.text(text))
    suspend fun send(c: Content): String = api.send(room.id, c)
    /** "입력 중" 10초 (ai면 "AI가 답을 쓰는 중") */
    suspend fun typing(ai: Boolean = false) = api.typing(room.id, ai)
    /** 관리 권한 */
    suspend fun delete() { api.call("delete", JSONObject().put("room", room.id).put("message", message.id)) }
    suspend fun kick() { api.call("kick", JSONObject().put("room", room.id).put("user", user.id)) }
    suspend fun timeout(minutes: Int = 10, reason: String = "") = api.timeout(room.id, user.id, minutes, reason)

    // Java에서 쓰기 쉬운 기다리는 판
    fun replyBlocking(c: Content): String = runBlocking { reply(c) }
    fun sendBlocking(c: Content): String = runBlocking { send(c) }
    fun typingBlocking(ai: Boolean) = runBlocking { typing(ai) }
    fun deleteBlocking() = runBlocking { delete() }
    fun kickBlocking() = runBlocking { kick() }
    fun timeoutBlocking(minutes: Int, reason: String) = runBlocking { timeout(minutes, reason) }
}

/** Java용 처리기 */
fun interface JavaHandler { @Throws(Exception::class) fun handle(ctx: Context) }

/** 봇 하나 */
class LudamBot @JvmOverloads constructor(
    private val token: String,
    /** 켤 때 명령어를 서버에 등록 */
    var syncCommands: Boolean = true,
    /** 끊기면 다시 연결 */
    var reconnect: Boolean = true,
) {
    private val http = OkHttpClient.Builder().connectTimeout(15, TimeUnit.SECONDS).readTimeout(0, TimeUnit.MILLISECONDS).pingInterval(0, TimeUnit.SECONDS).build()
    val api = LudamApi(token, http)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val commands = LinkedHashMap<String, Pair<String, suspend (Context) -> Unit>>()
    private val handlers = HashMap<String, MutableList<suspend (Context) -> Unit>>()
    private val ready = ArrayList<(BotUser) -> Unit>()
    var onError: (Throwable) -> Unit = { System.err.println("[루담 봇] ${it.message}") }
    @Volatile private var stopped = false
    @Volatile private var ws: WebSocket? = null

    /** /명령어 등록 (한글·영어 소문자·숫자·_ 20자까지) */
    fun command(name: String, description: String = "", run: suspend (Context) -> Unit) {
        val n = name.removePrefix("/").lowercase()
        require(CMD_RE.matches(n)) { "명령어 이름이 규칙에 맞지 않아요: $name" }
        commands[n] = description.take(60) to run
    }
    fun commandJava(name: String, description: String, h: JavaHandler) = command(name, description) { h.handle(it) }
    private fun on(type: String, run: suspend (Context) -> Unit) { handlers.getOrPut(type) { ArrayList() }.add(run) }
    fun onReady(f: (BotUser) -> Unit) { ready.add(f) }
    /** "모든 메시지" 권한 */
    fun onMessage(run: suspend (Context) -> Unit) = on("message", run)
    fun onMemberJoin(run: suspend (Context) -> Unit) = on("member_join", run)
    fun onMemberLeave(run: suspend (Context) -> Unit) = on("member_leave", run)
    fun onInstall(run: suspend (Context) -> Unit) = on("install", run)
    fun onMessageJava(h: JavaHandler) = onMessage { h.handle(it) }
    fun onMemberJoinJava(h: JavaHandler) = onMemberJoin { h.handle(it) }

    /** 봇 켜기 (끌 때까지 기다려요) */
    fun run() = runBlocking { start(); while (!stopped) delay(1000) }

    /** 봇 켜기 (기다리지 않아요) */
    suspend fun start(): Job {
        require(token.startsWith("ldb_")) { "봇 열쇠(token)가 필요해요. 개발자 사이트에서 받은 ldb_… 값을 넣어 주세요" }
        if (syncCommands && commands.isNotEmpty()) {
            runCatching {
                api.call("commands", JSONObject().put("commands", JSONArray().apply { commands.forEach { (n, v) -> put(JSONObject().put("name", n).put("description", v.first)) } }))
            }.onFailure { onError(Exception("명령어 맞추기 실패: ${it.message}", it)) }
        }
        return scope.launch {
            var tries = 0
            while (isActive && !stopped) {
                val closed = kotlinx.coroutines.CompletableDeferred<Int>()
                val req = Request.Builder().url(LUDAM_GATEWAY).header("Authorization", "Bot $token").header("User-Agent", "LudamBotKotlin/$LUDAM_VERSION").build()
                var beat: Job? = null
                ws = http.newWebSocket(req, object : WebSocketListener() {
                    override fun onMessage(webSocket: WebSocket, text: String) {
                        val m = runCatching { JSONObject(text) }.getOrNull() ?: return
                        when (m.optString("op")) {
                            "ready" -> {
                                tries = 0
                                val gap = m.optLong("heartbeat", 30000)
                                beat?.cancel()
                                beat = scope.launch { while (isActive) { delay(gap); webSocket.send("{\"op\":\"ping\"}") } }
                                val b = m.optJSONObject("bot")
                                ready.forEach { f -> runCatching { f(BotUser(b?.optString("id").orEmpty(), b?.optString("name").orEmpty())) } }
                            }
                            "event" -> m.optJSONObject("d")?.let { d -> scope.launch { dispatch(Context(api, d)) } }
                        }
                    }
                    override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { closed.complete(code) }
                    override fun onClosing(webSocket: WebSocket, code: Int, reason: String) { webSocket.close(code, null) }
                    override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                        if (response?.code == 401) { onError(LudamException(401, "auth", "봇 열쇠가 틀렸어요. 개발자 사이트에서 확인해 주세요")); stopped = true }
                        else onError(t)
                        closed.complete(response?.code ?: -1)
                    }
                })
                val code = closed.await()
                beat?.cancel()
                if (code == 4004) { onError(LudamException(4004, "token_reset", "열쇠를 새로 받아서 연결이 끊겼어요")); stopped = true }
                if (stopped || !reconnect) break
                delay(minOf(60_000L, 2000L shl minOf(tries++, 5)) + (0..999).random())
            }
        }
    }

    private suspend fun dispatch(ctx: Context) {
        handlers[ctx.type]?.forEach { h -> runCatching { h(ctx) }.onFailure(onError) }
        if (ctx.type == "command") commands[ctx.command.lowercase()]?.let { (_, run) -> runCatching { run(ctx) }.onFailure(onError) }
    }

    /** 봇 끄기 */
    fun stop() {
        stopped = true
        runCatching { ws?.close(1000, "bye") }
    }
}
