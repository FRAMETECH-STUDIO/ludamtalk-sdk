import com.ludamtalk.bot.LudamBot
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlin.test.Test
import kotlin.test.assertTrue

class FakeTokenTest {
    @Test fun badToken() = runBlocking {
        var got = ""
        val bot = LudamBot("ldb_AAAAAAAAAAAAAAAAAAAA_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB", syncCommands = false)
        bot.onError = { got = it.message ?: "" }
        bot.command("주사위", "x") { it.reply("1") }
        val job = bot.start()
        repeat(40) { if (got.isEmpty()) delay(250) }
        println("받은 오류: $got")
        job.cancel(); bot.stop()
        assertTrue(got.contains("열쇠"))
    }
}