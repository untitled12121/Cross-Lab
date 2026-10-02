package dev.crosslab.android.features.audit

import dev.crosslab.android.features.devices.*
import dev.crosslab.android.features.filetransfer.*
import dev.crosslab.android.features.pairing.PairingJoinerStage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.crosslab_mobile_ffi.MobileAuditAction
import uniffi.crosslab_mobile_ffi.MobileAuditOutcome

class AuditLifecycleTrackerTest {
    @Test fun transitionsProduceOnlyRedactedTypedIntentsOnce() {
        val tracker = AuditLifecycleTracker()
        assertTrue(tracker.pairing(PairingJoinerStage.IDLE).isEmpty())
        assertEquals(MobileAuditAction.PAIRING_STARTED,
            tracker.pairing(PairingJoinerStage.FINDING_DEVICE).single().action)
        // A second QR scan while discovery is active is a distinct owner attempt.
        assertEquals(MobileAuditAction.PAIRING_STARTED,
            tracker.pairing(PairingJoinerStage.FINDING_DEVICE).single().action)
        assertEquals(MobileAuditAction.PEER_TRUSTED,
            tracker.pairing(PairingJoinerStage.FINALIZING).single().action)
        assertEquals(MobileAuditAction.PAIRING_COMPLETED,
            tracker.pairing(PairingJoinerStage.PAIRED).single().action)
        assertEquals(MobileAuditAction.PAIRING_FAILED,
            tracker.pairing(PairingJoinerStage.FAILED).single().action)

        val connected = RuntimeSnapshot.disconnected().copy(
            session = RuntimeSession.ACTIVE,
            trust = RuntimeTrust.TRUSTED,
            security = RuntimeSecurity.AUTHENTICATED,
            policyRevision = 5uL,
            peerDeviceId = "private-device-id",
        )
        assertEquals(MobileAuditAction.SESSION_AUTHENTICATED,
            tracker.runtime(connected).single().action)
        assertTrue(tracker.runtime(connected).isEmpty())
        assertEquals(MobileAuditAction.PERMISSION_CHANGED,
            tracker.runtime(connected.copy(policyRevision = 6uL)).single().action)
        assertEquals(MobileAuditAction.SESSION_DISCONNECTED,
            tracker.runtime(RuntimeSnapshot.disconnected()).single().action)
        assertTrue(tracker.runtime(RuntimeSnapshot.disconnected()).isEmpty())
        assertEquals(MobileAuditAction.SESSION_AUTHENTICATED,
            tracker.runtime(connected.copy(policyRevision = 12uL)).single().action)
        val initial = FileTransferState(available = true)
        assertTrue(tracker.transfer(initial).isEmpty())
        tracker.transfer(initial.copy(stage = FileTransferStage.TRANSFERRING, displayName = "PRIVATE.txt"))
        val end = tracker.transfer(initial.copy(
            stage = FileTransferStage.FAILED, failure = FileTransferFailure.DENIED))
        assertEquals(MobileAuditAction.TRANSFER_ENDED, end.single().action)
        assertEquals(MobileAuditOutcome.DENIED, end.single().outcome)
        assertTrue(tracker.transfer(initial.copy(stage = FileTransferStage.FAILED)).isEmpty())
        assertEquals(MobileAuditAction.NOTIFICATION_SUBSCRIPTION_ENABLED,
            tracker.subscription(true).single().action)
        assertTrue(tracker.subscription(true).isEmpty())
        assertEquals(MobileAuditAction.NOTIFICATION_SUBSCRIPTION_DISABLED,
            tracker.subscription(false).single().action)
        assertEquals(0uL, end.single().revision)
        assertTrue(end.single().toString().contains("TRANSFER_ENDED"))
        assertTrue(!end.single().toString().contains("PRIVATE"))
    }
}
