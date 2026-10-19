package dev.crosslab.android.features.filetransfer

import java.io.IOException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FileTransferFailureTest {
    @Test
    fun failed_transfer_offers_an_actionable_but_non_sensitive_recovery_message() {
        val sample =
            FileTransferState(
                available = true,
                stage = FileTransferStage.FAILED,
                failure = FileTransferFailure.STORAGE,
            )
        assertTrue(checkNotNull(failureRecoveryHint(sample)).contains("document permissions"))
        assertNull(failureRecoveryHint(sample.copy(stage = FileTransferStage.TRANSFERRING)))
        assertNull(failureRecoveryHint(sample.copy(failure = null)))
        assertTrue(
            checkNotNull(failureRecoveryHint(sample.copy(failure = FileTransferFailure.INTEGRITY)))
                .contains("verified transfer"),
        )
    }

    @Test
    fun local_io_and_uri_permission_failures_are_not_network_failures() {
        assertEquals(
            FileTransferFailure.STORAGE,
            fileTransferFailure(IOException("source read failed"), FileTransferFailure.NETWORK),
        )
        assertEquals(
            FileTransferFailure.STORAGE,
            fileTransferFailure(SecurityException("URI permission revoked"), FileTransferFailure.NETWORK),
        )
        assertEquals(
            FileTransferFailure.NETWORK,
            fileTransferFailure(IllegalStateException("unknown"), FileTransferFailure.NETWORK),
        )
    }
}
