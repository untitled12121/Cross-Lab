package dev.crosslab.android.features.devices

/** Presentation is advisory; Rust still authorizes every exact protected operation. */
internal fun RuntimeSnapshot.hasAuthenticatedCapability(capabilityId: String): Boolean =
    presence == RuntimePresence.ONLINE &&
        !peerDeviceId.isNullOrBlank() &&
        trust == RuntimeTrust.TRUSTED &&
        connectivity == RuntimeConnectivity.CONNECTED &&
        session == RuntimeSession.ACTIVE &&
        security == RuntimeSecurity.AUTHENTICATED &&
        capabilityId in capabilityIds

/** Prevent native actions on stale or untrusted device snapshots. */
internal fun DevicePresentation.hasAuthenticatedSession(presence: PresenceDisplay): Boolean =
    presence == PresenceDisplay.ONLINE &&
        trust == TrustDisplay.TRUSTED &&
        connectivity == ConnectivityDisplay.CONNECTED &&
        session == SessionDisplay.ACTIVE &&
        security == SecurityDisplay.AUTHENTICATED

internal fun DevicePresentation.hasAuthenticatedCapability(
    capabilityId: String,
    presence: PresenceDisplay,
): Boolean = hasAuthenticatedSession(presence) && capabilityId in capabilityIds
