package dev.crosslab.android.features.audit

import dev.crosslab.android.features.devices.RuntimeSecurity
import dev.crosslab.android.features.devices.RuntimeSession
import dev.crosslab.android.features.devices.RuntimeSnapshot
import dev.crosslab.android.features.devices.RuntimeTrust
import dev.crosslab.android.features.filetransfer.FileTransferStage
import dev.crosslab.android.features.filetransfer.FileTransferState
import dev.crosslab.android.features.filetransfer.FileTransferFailure
import dev.crosslab.android.features.pairing.PairingJoinerStage
import uniffi.crosslab_mobile_ffi.MobileAuditAction
import uniffi.crosslab_mobile_ffi.MobileAuditOutcome

internal data class AuditIntent(
    val action: MobileAuditAction,
    val outcome: MobileAuditOutcome,
    val revision: ULong = 0uL,
)

/** Metadata-only transition classifier: neither full snapshots nor identifiers are retained. */
internal class AuditLifecycleTracker {
    private var previousPairing = PairingJoinerStage.IDLE
    private var previousSession = false
    private var previousPolicyRevision: ULong? = null
    private var lastRecordedPolicyRevision: ULong? = null
    private var previousTransfer = FileTransferStage.IDLE
    private var previousSubscription = false

    @Synchronized
    fun pairing(stage: PairingJoinerStage): List<AuditIntent> {
        // Every new QR attempt emits FINDING_DEVICE, even after replacing an active attempt.
        if (stage == previousPairing && stage != PairingJoinerStage.FINDING_DEVICE) {
            return emptyList()
        }
        previousPairing = stage
        return when (stage) {
            PairingJoinerStage.FINDING_DEVICE ->
                listOf(AuditIntent(MobileAuditAction.PAIRING_STARTED, MobileAuditOutcome.SUCCEEDED))
            PairingJoinerStage.FINALIZING ->
                listOf(AuditIntent(MobileAuditAction.PEER_TRUSTED, MobileAuditOutcome.SUCCEEDED))
            PairingJoinerStage.PAIRED ->
                listOf(AuditIntent(MobileAuditAction.PAIRING_COMPLETED, MobileAuditOutcome.SUCCEEDED))
            PairingJoinerStage.FAILED ->
                listOf(AuditIntent(MobileAuditAction.PAIRING_FAILED, MobileAuditOutcome.FAILED))
            PairingJoinerStage.CANCELLED ->
                listOf(AuditIntent(MobileAuditAction.PAIRING_CANCELLED, MobileAuditOutcome.CANCELLED))
            else -> emptyList()
        }
    }

    @Synchronized
    fun runtime(snapshot: RuntimeSnapshot): List<AuditIntent> {
        val events = mutableListOf<AuditIntent>()
        val active = snapshot.session == RuntimeSession.ACTIVE &&
            snapshot.security == RuntimeSecurity.AUTHENTICATED &&
            snapshot.trust == RuntimeTrust.TRUSTED
        if (active != previousSession) {
            events += AuditIntent(
                if (active) MobileAuditAction.SESSION_AUTHENTICATED
                else MobileAuditAction.SESSION_DISCONNECTED,
                MobileAuditOutcome.SUCCEEDED,
            )
            previousSession = active
        }
        val prior = previousPolicyRevision
        if (active) {
            if (prior != null && snapshot.policyRevision > prior) {
                events += policyRevisionEvent(snapshot.policyRevision)
            }
            previousPolicyRevision = snapshot.policyRevision
        } else {
            // Reconnection reloads already-authorized policy without a new edit.
            previousPolicyRevision = null
        }
        return events
    }

    /** A durable edit must still be recorded if its session closed during reload. */
    @Synchronized
    fun permissionCommitted(revision: ULong): List<AuditIntent> {
        previousPolicyRevision = revision
        return policyRevisionEvent(revision)
    }

    // Invoked only by the synchronized transition handlers.
    private fun policyRevisionEvent(revision: ULong): List<AuditIntent> {
        val recorded = lastRecordedPolicyRevision
        if (recorded != null && revision <= recorded) return emptyList()
        lastRecordedPolicyRevision = revision
        return listOf(
            AuditIntent(
                MobileAuditAction.PERMISSION_CHANGED,
                MobileAuditOutcome.SUCCEEDED,
                revision,
            ),
        )
    }

    @Synchronized
    fun transfer(state: FileTransferState): List<AuditIntent> {
        val stage = state.stage
        val prior = previousTransfer
        previousTransfer = stage
        val terminal = stage == FileTransferStage.COMPLETED ||
            stage == FileTransferStage.CANCELLED || stage == FileTransferStage.FAILED
        if (stage == prior || prior == FileTransferStage.IDLE || !terminal) {
            return emptyList()
        }
        val outcome = when (stage) {
            FileTransferStage.COMPLETED -> MobileAuditOutcome.SUCCEEDED
            FileTransferStage.CANCELLED -> MobileAuditOutcome.CANCELLED
            else -> if (state.failure == FileTransferFailure.DENIED)
                MobileAuditOutcome.DENIED else MobileAuditOutcome.FAILED
        }
        return listOf(AuditIntent(MobileAuditAction.TRANSFER_ENDED, outcome))
    }

    @Synchronized
    fun subscription(active: Boolean): List<AuditIntent> {
        if (previousSubscription == active) return emptyList()
        previousSubscription = active
        return listOf(AuditIntent(
            if (active) MobileAuditAction.NOTIFICATION_SUBSCRIPTION_ENABLED
            else MobileAuditAction.NOTIFICATION_SUBSCRIPTION_DISABLED,
            MobileAuditOutcome.SUCCEEDED,
        ))
    }
}
