package top.flysoftbeta.workflow.core.resource

/** A pending attachment: a workspace file (imports are written into the workspace first). */
data class ComposerAttachment(val path: String, val mimeType: String? = null)

/**
 * Unsent conversation input (text + pending attachments), keyed by conversation id and shared by
 * every session. Revision semantics:
 * - [edit] advances [revision] only when something changed; attachments are de-duplicated by path;
 * - [acknowledge] (after sending) clears only if owner, revision and content all match exactly;
 * - an empty draft with a revision is a tombstone that keeps revisions monotonic.
 */
data class ComposerDraft(
    val conversationId: String,
    val revision: Long = 0,
    val text: String = "",
    val attachments: List<ComposerAttachment> = emptyList(),
) {
    init { require(revision >= 0) }

    val hasContent: Boolean get() = text.isNotEmpty() || attachments.isNotEmpty()

    fun edit(text: String = this.text, attachments: List<ComposerAttachment> = this.attachments): ComposerDraft {
        val unique = attachments.distinctBy { it.path }
        if (text == this.text && unique == this.attachments) return this
        check(revision < Long.MAX_VALUE) { "Composer revision overflow" }
        return copy(revision = revision + 1, text = text, attachments = unique)
    }

    fun acknowledge(submitted: ComposerDraft): ComposerDraft? {
        if (conversationId != submitted.conversationId || revision != submitted.revision ||
            text != submitted.text || attachments != submitted.attachments) return null
        return edit(text = "", attachments = emptyList())
    }
}

class ComposerRevisionConflictException(val conversationId: String) :
    IllegalStateException("Conversation composer changed before it could be saved")
