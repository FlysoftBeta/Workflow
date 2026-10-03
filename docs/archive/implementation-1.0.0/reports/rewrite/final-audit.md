# Independent final boundary audit

2026-10-03. Read-only production review of Rust storage/workspace/environment/process ownership and the Android connection, Engine adapters, agent index, MainActivity/AppGraph lifecycle. Read `AGENTS.md`, `docs/protocol.md`, and verification guidance. No production source changed, no Cargo/Gradle build or device run, and no user transcripts inspected. This audit does not establish release or device acceptance.

## Finding: P2 — backend index refresh can overwrite an acknowledged user rename/archive

`app/src/main/java/top/flysoftbeta/workflow/platform/agent/AgentHub.kt:459–463` reads an entry outside `ConversationIndexFile`'s edit mutex, creates a refreshed copy, and calls `index.upsert(merged)`. `ConversationIndexFile.kt:126–130` serializes the *write* but replaces the entire entry with the caller's already-stale copy. The existing `index.update` method provides the correct transform-under-lock primitive, but the background writer bypasses it.

A normal Engine document write suspends while the user operation holds `editLock`. During that suspension, `mutable.value` still exposes the old entry (`publish` updates it only after `store.writeConversationIndex` returns). A coalesced backend event can therefore read the old title/archive/model fields, wait behind the user write, and then durably overwrite the user's acknowledged result. This is a client-to-authoritative-document lost update; the Server cannot reject it because the caller sends a complete replacement document without a document CAS.

### Deterministic reproduction performed

A lightweight Java harness used the already-built production `ConversationIndexFile` classes and Kotlin coroutine/serialization libraries. It supplied a `WorkspaceStore` proxy only to gate one document-write acknowledgement, reproducing the exact `maintainIndex` snapshot → upsert sequence. The harness did not start an Engine or vendor backend and is not claimed as device acceptance.

1. Insert conversation `a`, thread `thread-a`, no title.
2. Start `index.update("a") { ...title = "USER TITLE" }`; pause its `writeConversationIndex` before acknowledgement while it holds the edit mutex.
3. Read `index.entries`: it still has no title, as `AgentHub.maintainIndex` does. Queue the backend-refreshed copy with title `BACKEND TITLE` through `index.upsert`.
4. Release the user-write acknowledgement and await both operations.
5. Both calls succeed, but the final projected and written document title is `BACKEND TITLE`.

Evidence:

- Harness: `artifacts/final-audit/IndexRace.java`.
- Output: `artifacts/final-audit/index-race.log`.
- Actual output: `user rename acknowledged; final projected title=BACKEND TITLE` and `Engine document would persist BACKEND TITLE=true`.

The current `ConversationIndexFileTest.concurrentUpdatesSurviveStoreFlushAndReopen` exercises only concurrent `update` calls, so it does not cover the remaining `maintainIndex` upsert path.

### Small suggested repair

Keep the entry lookup as an ID lookup, then perform the refresh and user-field preservation inside `index.update`:

```kotlin
val entry = conversations.value.firstOrNull {
    it.backend == key.backend && it.backendThreadId == key.id
} ?: continue
index.update(entry.id) { current ->
    if (current.backend != key.backend || current.backendThreadId != key.id) current
    else {
        val refreshed = ConversationIndexing.refresh(current, snapshot.thread(key), now)
        refreshed.copy(title = current.title ?: refreshed.title, archived = current.archived)
    }
}
```

The existing helper already returns without recreating a removed entry. A regression should gate `writeConversationIndex` as above, queue the actual background refresh while the user write is pending, then assert the explicit title/archive changes survive in both the in-memory projection and reopened document. Preserve intended model/effort semantics when adding that assertion, because `ConversationIndexing.refresh` intentionally reads backend turn settings.

## Other reviewed paths

No additional concrete finding was established in the inspected no-replace create/upload/move/restore paths, workspace draft/archive separation, environment generation/home activation, process ownership, connection retirement, or recent MainActivity/TerminalHost integration repairs. This statement is limited to this source audit and the small race reproduction; the pending final release verification remains separate.

## Integration resolution

Root replaced the background upsert with `ConversationIndexFile.refreshThread`, which computes its merge inside `update`’s mutex. The deterministic delayed Engine acknowledgement regression now preserves both the user title and archive flag in projection and reopened document, and cannot recreate a removed entry. `artifacts/rewrite-integration/index-race-fix.log`: all 3 index tests passed.
