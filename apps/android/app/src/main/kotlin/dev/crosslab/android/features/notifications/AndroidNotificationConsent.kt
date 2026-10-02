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

class AndroidNotificationConsent(context: Context) {
    private val application = context.applicationContext
    private val preferences =
        application.getSharedPreferences("crosslab.notifications.owner-v1", Context.MODE_PRIVATE)

    val ownerEnabled: Boolean
        get() = preferences.getBoolean("owner-enabled", false)

    val contentEnabled: Boolean
        get() = preferences.getBoolean("content-enabled", false) && ownerEnabled

    fun setOwnerEnabled(enabled: Boolean) {
        preferences.edit()
            .putBoolean("owner-enabled", enabled)
            .apply()
        if (!enabled) setContentEnabled(false)
    }

    fun setContentEnabled(enabled: Boolean) {
        preferences.edit()
            .putBoolean("content-enabled", enabled && ownerEnabled)
            .apply()
    }

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
            Settings.Secure.getString(
                application.contentResolver,
                "enabled_notification_listeners",
            ).orEmpty()
        return enabled
            .split(':')
            .mapNotNull(ComponentName::unflattenFromString)
            .any { it == component }
    }

    companion object {
        internal val listenerConnected = AtomicBoolean(false)
    }
}
