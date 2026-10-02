package dev.crosslab.android.features.notifications

import android.service.notification.NotificationListenerService

/**
 * OS callback boundary. Do not inspect or retain notifications before an authenticated,
 * policy-approved session subscriber is connected to the Rust capability runtime.
 */
class CrossLabNotificationListener : NotificationListenerService() {
    override fun onListenerConnected() {
        AndroidNotificationConsent.listenerConnected.set(true)
    }

    override fun onListenerDisconnected() {
        AndroidNotificationConsent.listenerConnected.set(false)
    }

    override fun onDestroy() {
        AndroidNotificationConsent.listenerConnected.set(false)
        super.onDestroy()
    }
}
