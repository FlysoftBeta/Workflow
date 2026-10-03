package top.flysoftbeta.workflow.agent.model

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class ModelSupportTest {
    @Test fun streamTextAppendsAndCompacts() {
        var text = StreamText.EMPTY
        val expected = StringBuilder()
        repeat(2000) { i -> text = text.append("$i,"); expected.append("$i,") }
        assertEquals(expected.toString(), text.toString())
        assertEquals(expected.length, text.length)
        val snapshot = text
        val longer = text.append("x")
        assertEquals(expected.toString(), snapshot.toString()) // older snapshots are unaffected
        assertEquals("$expected" + "x", longer.toString())
        assertEquals(StreamText.of("ab"), StreamText.of("a").append("b"))
        assertSame(text, text.append(""))
        assertEquals("c", StreamText.of("abc").takeLast(1).toString())
    }

    @Test fun conversationIndexRefreshSearchAndOrder() = runBlocking {
        val index = InMemoryConversationIndex()
        val entry = ConversationEntry("c1", BackendKind.CLAUDE, null, null, "/workspace", 1, 1)
        index.upsert(entry)
        val thread = ThreadState(
            ThreadKey(BackendKind.CLAUDE, "s1"), title = "Refactor parser",
            turns = listOf(Turn("t", items = listOf(UserMessageItem("u", listOf(UserPart.Text("please refactor the parser")))), settings = TurnSettings("sonnet", "high"))),
        )
        val refreshed = ConversationIndexing.refresh(entry, thread, nowMs = 50)
        assertEquals("s1", refreshed.backendThreadId)
        assertEquals("Refactor parser", refreshed.title)
        assertEquals("sonnet", refreshed.model)
        assertEquals("high", refreshed.effort)
        assertEquals(50, refreshed.updatedAtMs)
        assertSame(refreshed, ConversationIndexing.refresh(refreshed, thread, nowMs = 99))
        index.upsert(refreshed)
        index.upsert(ConversationEntry("c2", BackendKind.CODEX, "t2", "Old", "/workspace", 1, 2, archived = true))
        index.upsert(ConversationEntry("c3", BackendKind.CODEX, "t3", "Newest", "/workspace", 1, 100))
        assertEquals(listOf("c3", "c1", "c2"), ConversationIndexing.sorted(index.entries.value).map { it.id })
        assertEquals(listOf("c1"), ConversationIndexing.search(index.entries.value, "PARSER").map { it.id })
        index.remove("c2")
        assertTrue(index.entries.value.none { it.id == "c2" })
    }
}
