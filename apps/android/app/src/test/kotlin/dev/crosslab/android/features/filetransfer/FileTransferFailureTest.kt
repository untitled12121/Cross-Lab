package dev.crosslab.android.features.filetransfer

import java.io.IOException
import org.junit.Assert.assertEquals
import org.junit.Test

class FileTransferFailureTest {
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
