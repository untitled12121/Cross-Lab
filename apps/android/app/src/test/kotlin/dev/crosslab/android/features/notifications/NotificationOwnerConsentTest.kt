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

    private class FakeOwnerConsentStorage : OwnerConsentStorage {
        var owner = false
        var content = false
        var failNext = false
        var throwNext = false
        var failRead = false
        var duringCommit: (() -> Unit)? = null

        override fun ownerEnabled(): Boolean {
            if (failRead) throw IllegalStateException("read unavailable")
            return owner
        }
        override fun contentEnabled() = content

        override fun commitOwner(enabled: Boolean): Boolean {
            if (throwNext) {
                throwNext = false
                throw IllegalStateException("storage unavailable")
            }
            if (failNext) {
                failNext = false
                return false
            }
            owner = enabled
            content = false
            duringCommit?.invoke()
            return true
        }

        override fun commitContent(enabled: Boolean): Boolean {
            if (throwNext) {
                throwNext = false
                throw IllegalStateException("storage unavailable")
            }
            if (failNext) {
                failNext = false
                return false
            }
            content = enabled
            duringCommit?.invoke()
            return true
        }
    }

    @Test
    fun ownerReapprovalDoesNotRestorePreviouslySharedNotificationContents() {
        val store = FakeOwnerConsentStorage()
        val consent = OwnerConsentSettings(store)
        assertFalse(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)

        assertTrue(consent.setOwnerEnabled(true))
        assertTrue(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)
        assertTrue(consent.setContentEnabled(true))
        assertTrue(consent.contentEnabled)

        assertTrue(consent.setOwnerEnabled(false))
        assertFalse(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)
        assertFalse(store.content)

        assertTrue(consent.setOwnerEnabled(true))
        assertTrue(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)
    }

    @Test
    fun disablingContentPreservesRedactedMirroringAfterDurableCommit() {
        val store = FakeOwnerConsentStorage()
        val consent = OwnerConsentSettings(store)
        assertTrue(consent.setOwnerEnabled(true))
        assertTrue(consent.setContentEnabled(true))

        assertTrue(consent.setContentEnabled(false))
        assertTrue(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)
        assertFalse(store.content)
    }

    @Test
    fun failedOwnerConsentWithdrawalImmediatelyBlocksContentEvenIfStorageStaysEnabled() {
        val store = FakeOwnerConsentStorage()
        val consent = OwnerConsentSettings(store)
        assertTrue(consent.setOwnerEnabled(true))
        assertTrue(consent.setContentEnabled(true))
        store.failNext = true

        assertFalse(consent.setOwnerEnabled(false))
        assertTrue(store.owner)
        assertTrue(store.content)
        assertFalse(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)

        assertTrue(consent.setOwnerEnabled(true))
        assertFalse(consent.contentEnabled)
    }

    @Test
    fun failedContentRevocationAlsoBlocksAllMirroringUntilFreshOwnerApproval() {
        val store = FakeOwnerConsentStorage()
        val consent = OwnerConsentSettings(store)
        assertTrue(consent.setOwnerEnabled(true))
        assertTrue(consent.setContentEnabled(true))
        store.throwNext = true

        assertFalse(consent.setContentEnabled(false))
        assertFalse(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)
        assertTrue(store.content)

        assertTrue(consent.setOwnerEnabled(true))
        assertFalse(consent.contentEnabled)
    }

    @Test
    fun consentRemainsBlockedUntilEachEnablingWriteReturnsDurableSuccess() {
        val store = FakeOwnerConsentStorage()
        val consent = OwnerConsentSettings(store)
        var exposedDuringCommit = false
        store.duringCommit = {
            exposedDuringCommit = exposedDuringCommit ||
                consent.ownerEnabled || consent.contentEnabled
        }

        assertTrue(consent.setOwnerEnabled(true))
        assertTrue(consent.setContentEnabled(true))
        assertFalse(exposedDuringCommit)
        assertTrue(consent.contentEnabled)
    }

    @Test
    fun preferenceReadFailureBlocksMirroringUntilFreshOwnerApproval() {
        val store = FakeOwnerConsentStorage()
        val consent = OwnerConsentSettings(store)
        assertTrue(consent.setOwnerEnabled(true))
        store.failRead = true

        assertFalse(consent.ownerEnabled)
        store.failRead = false
        assertFalse(consent.ownerEnabled)

        assertTrue(consent.setOwnerEnabled(true))
        assertTrue(consent.ownerEnabled)
        assertFalse(consent.contentEnabled)
    }

    @Test
    fun contentCannotBeAuthorizedWithoutOwnerPermission() {
        val store = FakeOwnerConsentStorage()
        val consent = OwnerConsentSettings(store)
        assertTrue(consent.setContentEnabled(true))
        assertFalse(store.content)
        assertFalse(consent.contentEnabled)
    }
}
