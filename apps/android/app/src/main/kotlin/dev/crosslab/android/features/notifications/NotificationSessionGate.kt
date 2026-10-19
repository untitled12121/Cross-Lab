package dev.crosslab.android.features.notifications

import dev.crosslab.android.features.devices.RuntimeConnectivity
import dev.crosslab.android.features.devices.RuntimePresence
import dev.crosslab.android.features.devices.RuntimeSecurity
import dev.crosslab.android.features.devices.RuntimeSession
import dev.crosslab.android.features.devices.RuntimeSnapshot
import dev.crosslab.android.features.devices.RuntimeTrust

private const val NOTIFICATION_CAPABILITY = "notifications.read"

/** Native capture never treats durable trust or a published service as a live session. */
internal fun RuntimeSnapshot.canMirrorNotifications(): Boolean =
    presence == RuntimePresence.ONLINE &&
        !peerDeviceId.isNullOrBlank() &&
        trust == RuntimeTrust.TRUSTED &&
        connectivity == RuntimeConnectivity.CONNECTED &&
        session == RuntimeSession.ACTIVE &&
        security == RuntimeSecurity.AUTHENTICATED &&
        NOTIFICATION_CAPABILITY in capabilityIds
