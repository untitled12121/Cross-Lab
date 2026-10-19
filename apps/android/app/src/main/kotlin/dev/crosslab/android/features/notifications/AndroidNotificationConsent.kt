package dev.crosslab.android.features.notifications

import android.content.ComponentName
import android.content.Context
import android.provider.Settings
import java.util.concurrent.atomic.AtomicBoolean

data class NotificationOwnerConsent(
    val osAccess: Boolean,
    val listenerConnected: Boolean,
    val ownerEnabled: Boolean,
    val contentEnabled: Boolean,
    val foreground: Boolean,
) {
    // A separate authenticated session subscription and exact policy allow are also required.
    fun locallyAvailable(): Boolean =
        osAccess && listenerConnected && ownerEnabled && foreground

    fun permitsContent(): Boolean =
        locallyAvailable() && contentEnabled

    internal fun outgoingText(title: String?, preview: String?, protected: Boolean): NotificationText =
        if (protected || !permitsContent()) NotificationText(null, null)
        else NotificationText(title, preview)
}

internal data class NotificationText(val title: String?, val preview: String?)

/**
 * Local owner consent is independent of OS permission, authenticated sessions and peer policy.
 * Failure to persist an edit blocks mirroring in this process until explicit owner re-approval.
 */
internal interface OwnerConsentStorage {
    fun ownerEnabled(): Boolean
    fun contentEnabled(): Boolean
    fun commitOwner(enabled: Boolean): Boolean
    fun commitContent(enabled: Boolean): Boolean
}

internal class OwnerConsentSettings(
    private val store: OwnerConsentStorage,
) {
    private val lock = Any()
    private var blocked = false

    val ownerEnabled: Boolean
        get() =
            synchronized(lock) {
                if (blocked) return@synchronized false
                runCatching { store.ownerEnabled() }.getOrElse {
                    blocked = true
                    false
                }
            }

    val contentEnabled: Boolean
        get() =
            synchronized(lock) {
                if (!ownerEnabled) return@synchronized false
                runCatching { store.contentEnabled() }.getOrElse {
                    blocked = true
                    false
                }
            }

    fun <T> withConsent(block: (ownerEnabled: Boolean, contentEnabled: Boolean) -> T): T =
        synchronized(lock) {
            // Keep the consent lock until the outbound agent call completes.
            val owner = ownerEnabled
            block(owner, owner && contentEnabled)
        }

    fun setOwnerEnabled(enabled: Boolean): Boolean =
        synchronized(lock) {
            // Do not expose consent before both flags are durably committed.
            blocked = true
            val saved = runCatching { store.commitOwner(enabled) }.getOrDefault(false)
            blocked = !saved
            saved
        }

    fun setContentEnabled(enabled: Boolean): Boolean =
        synchronized(lock) {
            val consented = enabled && ownerEnabled
            val wasAvailable = !blocked
            blocked = true
            val saved = runCatching { store.commitContent(consented) }.getOrDefault(false)
            // A failed prior read/write requires fresh owner approval to unblock.
            blocked = !saved || !wasAvailable
            saved
        }
}

class AndroidNotificationConsent(context: Context) {
    private val application = context.applicationContext
    private val preferences =
        application.getSharedPreferences("crosslab.notifications.owner-v1", Context.MODE_PRIVATE)
    private val settings =
        OwnerConsentSettings(
            object : OwnerConsentStorage {
                override fun ownerEnabled() = preferences.getBoolean("owner-enabled", false)
                override fun contentEnabled() = preferences.getBoolean("content-enabled", false)

                override fun commitOwner(enabled: Boolean): Boolean =
                    preferences.edit()
                        .putBoolean("owner-enabled", enabled)
                        .putBoolean("content-enabled", false)
                        .commit()

                override fun commitContent(enabled: Boolean): Boolean =
                    preferences.edit().putBoolean("content-enabled", enabled).commit()
            },
        )

    val ownerEnabled: Boolean
        get() = settings.ownerEnabled

    val contentEnabled: Boolean
        get() = settings.contentEnabled

    fun setOwnerEnabled(enabled: Boolean): Boolean = settings.setOwnerEnabled(enabled)

    fun setContentEnabled(enabled: Boolean): Boolean = settings.setContentEnabled(enabled)

    fun current(foreground: Boolean): NotificationOwnerConsent =
        withCurrent(foreground) { it }

    fun <T> withCurrent(
        foreground: Boolean,
        block: (NotificationOwnerConsent) -> T,
    ): T =
        settings.withConsent { owner, content ->
            block(
                NotificationOwnerConsent(
                    osAccess = osAccessGranted(),
                    listenerConnected = listenerConnected.get(),
                    ownerEnabled = owner,
                    contentEnabled = content,
                    foreground = foreground,
                ),
            )
        }

    private fun osAccessGranted(): Boolean {
        val component =
            ComponentName(application, CrossLabNotificationListener::class.java)
        // Exact ComponentName comparison avoids substring matches with other packages.
        val enabled =
            runCatching {
                Settings.Secure.getString(
                    application.contentResolver,
                    "enabled_notification_listeners",
                )
            }.getOrNull().orEmpty()
        return enabled
            .split(':')
            .mapNotNull(ComponentName::unflattenFromString)
            .any { it == component }
    }

    companion object {
        internal val listenerConnected = AtomicBoolean(false)
    }
}
