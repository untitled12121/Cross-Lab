package dev.crosslab.android.features.notifications

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NotificationOwnerConsentTest {
    @Test
    fun everyIndependentOwnerAndPlatformGateMustBePresent() {
        val allowed = NotificationOwnerConsent(
            osAccess = true,
            listenerConnected = true,
            ownerEnabled = true,
            contentEnabled = false,
            foreground = true,
        )
        assertTrue(allowed.locallyAvailable())
        assertFalse(allowed.permitsContent())
        assertFalse(allowed.copy(osAccess = false).locallyAvailable())
        assertFalse(allowed.copy(listenerConnected = false).locallyAvailable())
        assertFalse(allowed.copy(ownerEnabled = false).locallyAvailable())
        assertFalse(allowed.copy(foreground = false).locallyAvailable())
        assertTrue(allowed.copy(contentEnabled = true).permitsContent())
    }
}
