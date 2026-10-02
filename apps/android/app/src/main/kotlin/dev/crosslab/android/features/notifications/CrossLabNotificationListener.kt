package dev.crosslab.android.features.notifications

import android.app.Notification
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import dev.crosslab.android.CrossLabApplication

/**
 * No notification fields are inspected before the Rust agent confirms a live
 * authenticated and policy-authorized subscription.
 */
class CrossLabNotificationListener : NotificationListenerService() {
    override fun onListenerConnected() {
        AndroidNotificationConsent.listenerConnected.set(true)
        (application as? CrossLabApplication)?.notificationPermissionChanged()
    }

    override fun onListenerDisconnected() {
        AndroidNotificationConsent.listenerConnected.set(false)
        (application as? CrossLabApplication)?.notificationPermissionChanged()
    }

    override fun onNotificationPosted(sbn: StatusBarNotification?) {
        val publisher =
            (application as? CrossLabApplication)?.notificationPublisher() ?: return
        if (!publisher.canCapture() || sbn == null) return

        val event = sbn.notification ?: return
        if (event.visibility == Notification.VISIBILITY_SECRET ||
            sbn.isOngoing ||
            event.flags and Notification.FLAG_FOREGROUND_SERVICE != 0
        ) {
            return
        }

        val key = sbn.key ?: return
        if (key.length !in 1..256) return
        val appLabel =
            runCatching {
                val info = packageManager.getApplicationInfo(sbn.packageName, 0)
                packageManager.getApplicationLabel(info).toString()
            }.getOrDefault("App")
        val label = boundedDisplay(appLabel, 96)

        // A private/system-marked notification never exposes title/preview.
        val includeContent =
            event.visibility == Notification.VISIBILITY_PUBLIC && publisher.contentAllowed()
        val title =
            if (includeContent) {
                boundedDisplay(event.extras?.getCharSequence(Notification.EXTRA_TITLE), 256)
            } else {
                null
            }
        val preview =
            if (includeContent) {
                boundedDisplay(event.extras?.getCharSequence(Notification.EXTRA_TEXT), 512)
            } else {
                null
            }
        publisher.posted(
            key,
            label.orEmpty(),
            title,
            preview,
            event.visibility != Notification.VISIBILITY_PUBLIC,
        )
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification?) {
        val publisher =
            (application as? CrossLabApplication)?.notificationPublisher() ?: return
        if (!publisher.canCapture() || sbn == null) return
        val key = sbn.key ?: return
        if (key.length in 1..256) publisher.removed(key)
    }

    override fun onDestroy() {
        AndroidNotificationConsent.listenerConnected.set(false)
        (application as? CrossLabApplication)?.notificationPermissionChanged()
        super.onDestroy()
    }
}

/** Bounded plain text only; skip controls and unpaired UTF-16 surrogates. */
internal fun boundedDisplay(input: CharSequence?, maxBytes: Int): String? {
    if (input == null) return null
    val output = StringBuilder()
    var byteCount = 0
    for (char in input) {
        if (char.isISOControl() || char.isSurrogate()) continue
        val size = char.toString().toByteArray(Charsets.UTF_8).size
        if (byteCount + size > maxBytes) break
        output.append(char)
        byteCount += size
    }
    return output.toString()
}
