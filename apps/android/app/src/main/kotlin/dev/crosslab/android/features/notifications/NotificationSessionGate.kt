package dev.crosslab.android.features.notifications

import dev.crosslab.android.features.devices.RuntimeSnapshot
import dev.crosslab.android.features.devices.hasAuthenticatedCapability

private const val NOTIFICATION_CAPABILITY = "notifications.read"

/** Native capture never treats durable trust or a published service as a live session. */
internal fun RuntimeSnapshot.canMirrorNotifications(): Boolean =
    hasAuthenticatedCapability(NOTIFICATION_CAPABILITY)
