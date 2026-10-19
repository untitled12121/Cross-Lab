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
}

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
    @Volatile private var unavailable = false

    val ownerEnabled: Boolean
        get() = !unavailable && store.ownerEnabled()

    val contentEnabled: Boolean
        get() = ownerEnabled && store.contentEnabled()

    fun setOwnerEnabled(enabled: Boolean): Boolean {
        // Disable capture before attempting a durable withdrawal.
        if (!enabled) unavailable = true
        val saved = runCatching { store.commitOwner(enabled) }.getOrDefault(false)
        unavailable = !saved
        return saved
    }

    fun setContentEnabled(enabled: Boolean): Boolean {
        val wasAvailable = !unavailable
        if (!enabled) unavailable = true
        val saved =
            runCatching { store.commitContent(enabled && ownerEnabled) }.getOrDefault(false)
        if (!saved) {
            unavailable = true
        } else if (!enabled && wasAvailable) {
            // A durable content opt-out keeps redacted mirroring available.
            unavailable = false
        }
        // Recovering from a prior storage failure requires fresh owner approval.
        return saved
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
        NotificationOwnerConsent(
            osAccess = osAccessGranted(),
            listenerConnected = listenerConnected.get(),
            ownerEnabled = ownerEnabled,
            contentEnabled = contentEnabled,
            foreground = foreground,
        )

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
