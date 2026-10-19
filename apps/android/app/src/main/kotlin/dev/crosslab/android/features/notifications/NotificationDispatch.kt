package dev.crosslab.android.features.notifications

/** Only constructed after the Rust-backed capture gate allows inspecting OS metadata. */
internal data class NotificationDelivery(
    val key: String,
    val appLabel: String,
    val title: String?,
    val preview: String?,
    val isProtected: Boolean,
)

/**
 * Native/OS callbacks are untrusted failure boundaries. Reject delivery on any
 * capture or consent check fault, and re-redact even a misbehaving extractor.
 */
internal class NotificationDispatch(private val publisher: NotificationPublisher) {
    fun posted(extract: (contentAllowed: Boolean) -> NotificationDelivery?): Boolean {
        if (!canCapture()) return false
        val contentAllowed = runCatching { publisher.contentAllowed() }.getOrNull() ?: return false
        val payload = runCatching { extract(contentAllowed) }.getOrNull() ?: return false
        val shareContent = contentAllowed && !payload.isProtected
        return runCatching {
            publisher.posted(
                key = payload.key,
                appLabel = payload.appLabel,
                title = if (shareContent) payload.title else null,
                preview = if (shareContent) payload.preview else null,
                protected = payload.isProtected,
            )
        }.getOrDefault(false)
    }

    fun removed(extractKey: () -> String?): Boolean {
        if (!canCapture()) return false
        val key = runCatching { extractKey() }.getOrNull() ?: return false
        return runCatching { publisher.removed(key) }.getOrDefault(false)
    }

    private fun canCapture(): Boolean =
        runCatching { publisher.canCapture() }.getOrDefault(false)
}
