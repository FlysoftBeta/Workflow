package top.flysoftbeta.workflow.feature.chat

import java.lang.reflect.Proxy
import androidx.compose.ui.text.input.TextFieldValue
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import top.flysoftbeta.workflow.core.resource.*
import top.flysoftbeta.workflow.core.store.*

class ComposerRevisionTest {
    @Test fun unchangedSavesAndFailedSendRestoreRetainThePersistedRevision() = runBlocking {
        Fixture(ComposerDraft("c", 7, "hello", listOf(ComposerAttachment("note.txt", "text/plain")))).useModel {
            assertTrue(model.loaded)
            assertEquals(7L, model.save().revision)
            assertEquals(7L, model.save().revision)
            val (text, files) = model.takeForSend()
            model.restore(text, files.map { it.path })
            val restored = model.save()
            assertEquals(7L, restored.revision)
            assertEquals("text/plain", restored.attachments.single().mimeType)
        }
    }

    @Test fun acknowledgementRetainsNewLocalTextAndAdvancesFromStoredBaseline() = runBlocking {
        Fixture(ComposerDraft("c", 3, "submitted")).useModel {
            val submitted = model.save()
            model.takeForSend()
            model.onTextChange(TextFieldValue("next local message"))
            val acknowledged = submitted.acknowledge(submitted)!!
            publish(acknowledged)
            model.accepted(submitted)
            assertEquals("next local message", model.text.text)
            val next = model.save()
            assertEquals(acknowledged.revision + 1, next.revision)
            assertEquals(next, model.save())
        }
    }

    @Test fun acknowledgementOfAlreadySavedNextMessageDoesNotInventAnotherRevision() = runBlocking {
        Fixture(ComposerDraft("c", 3, "submitted")).useModel {
            val submitted = model.save()
            model.takeForSend()
            model.onTextChange(TextFieldValue("next message"))
            val next = model.save()
            model.accepted(submitted) // Engine kept the newer composer rather than clearing it.
            assertEquals(next, model.save())
            assertEquals("next message", model.text.text)
        }
    }

    @Test fun adoptingConflictKeepsTheAuthoritativeBaselineForSubsequentSaves() = runBlocking {
        Fixture(ComposerDraft("c", 1, "old")).useModel {
            model.onTextChange(TextFieldValue("local"))
            val other = ComposerDraft("c", 2, "other session")
            publish(other)
            assertEquals(other, model.save())
            assertEquals(other, model.save())
        }
    }

    private class Fixture(initial: ComposerDraft) {
        private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        private val state = MutableStateFlow(WorkspaceState(status = StoreStatus.READY, composers = mapOf("c" to initial)))
        private val store = Proxy.newProxyInstance(WorkspaceStore::class.java.classLoader, arrayOf(WorkspaceStore::class.java)) { _, method, args ->
            when (method.name) {
                "getState" -> state
                "editComposer" -> {
                    val proposed = args[0] as ComposerDraft
                    val expected = args[1] as Long
                    val current = state.value.composer("c")
                    if (expected != current.revision) throw ComposerRevisionConflictException("c")
                    val stored = current.edit(proposed.text, proposed.attachments)
                    publish(stored)
                    stored
                }
                else -> error("Unexpected store call: ${method.name}")
            }
        } as WorkspaceStore
        private val services = Proxy.newProxyInstance(ChatFeatureServices::class.java.classLoader, arrayOf(ChatFeatureServices::class.java)) { _, method, _ ->
            error("No platform services needed by composer revision tests: ${method.name}")
        } as ChatFeatureServices
        val model = ComposerModel("c", store, services, scope)
        fun publish(draft: ComposerDraft) { state.value = state.value.copy(composers = mapOf("c" to draft)) }
        suspend fun useModel(block: suspend Fixture.() -> Unit) { try { block() } finally { scope.cancel() } }
    }
}
