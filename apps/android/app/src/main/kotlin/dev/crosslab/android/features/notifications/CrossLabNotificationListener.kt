package dev.crosslab.android.features.notifications

import android.app.Notification
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import dev.crosslab.android.CrossLabApplication

/**
 * Do not inspect notification payloads before a fresh authenticated,
 * owner-authorized subscription permits capture.
 */
class CrossLabNotificationListener : NotificationListenerService() {
    override fun onListenerConnected() {
        AndroidNotificationConsent.listenerConnected.set(true)
        runCatching { (application as? CrossLabApplication)?.notificationPermissionChanged() }
    }

    override fun onListenerDisconnected() {
        AndroidNotificationConsent.listenerConnected.set(false)
        runCatching { (application as? CrossLabApplication)?.notificationPermissionChanged() }
    }

    override fun onNotificationPosted(sbn: StatusBarNotification?) {
        val publisher =
            (application as? CrossLabApplication)?.notificationPublisher() ?: return
        if (sbn == null) return

        NotificationDispatch(publisher).posted { contentAllowed ->
            val event = sbn.notification ?: return@posted null
            if (excludedNotificationSource(sbn.packageName, event.category) ||
                event.visibility == Notification.VISIBILITY_SECRET ||
                sbn.isOngoing ||
                event.flags and Notification.FLAG_FOREGROUND_SERVICE != 0
            ) return@posted null

            val key = sbn.key ?: return@posted null
            if (key.length !in 1..256) return@posted null
            val appLabel =
                runCatching {
                    val info = packageManager.getApplicationInfo(sbn.packageName, 0)
                    packageManager.getApplicationLabel(info).toString()
                }.getOrDefault("App")
            val includeContent =
                event.visibility == Notification.VISIBILITY_PUBLIC && contentAllowed
            NotificationDelivery(
                key = key,
                appLabel = boundedDisplay(appLabel, 96).orEmpty(),
                title = if (includeContent) {
                    boundedDisplay(event.extras?.getCharSequence(Notification.EXTRA_TITLE), 256)
                } else null,
                preview = if (includeContent) {
                    boundedDisplay(event.extras?.getCharSequence(Notification.EXTRA_TEXT), 512)
                } else null,
                isProtected = event.visibility != Notification.VISIBILITY_PUBLIC,
            )
        }
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification?) {
        val publisher =
            (application as? CrossLabApplication)?.notificationPublisher() ?: return
        if (sbn == null) return
        NotificationDispatch(publisher).removed {
            sbn.key?.takeIf { it.length in 1..256 }
        }
    }

    override fun onDestroy() {
        AndroidNotificationConsent.listenerConnected.set(false)
        runCatching { (application as? CrossLabApplication)?.notificationPermissionChanged() }
        super.onDestroy()
    }
}

/** Reject common OS sources and protected service/call categories before capture. */
internal fun excludedNotificationSource(packageName: String, category: String?): Boolean =
    packageName == "android" ||
        packageName == "com.android.systemui" ||
        category == Notification.CATEGORY_CALL ||
        category == Notification.CATEGORY_SYSTEM ||
        category == Notification.CATEGORY_SERVICE

/** Bounded plain text only; skip controls and unpaired UTF-16 surrogates. */
internal fun boundedDisplay(input: CharSequence?, maxBytes: Int): String? {
    if (input == null) return null
    val output = StringBuilder()
    var byteCount = 0
    var index = 0
    while (index < input.length) {
        val codePoint = Character.codePointAt(input, index)
        index += Character.charCount(codePoint)
        if (codePoint in 0xD800..0xDFFF || Character.isISOControl(codePoint)) continue
        val size = when {
            codePoint < 0x80 -> 1
            codePoint < 0x800 -> 2
            codePoint < 0x10000 -> 3
            else -> 4
        }
        if (byteCount + size > maxBytes) break
        output.appendCodePoint(codePoint)
        byteCount += size
    }
    return output.toString()
}
