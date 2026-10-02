package dev.crosslab.android.features.filetransfer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class FileTransferControllerTest {
    @Test
    fun controller_observes_state_and_releases_subscription() {
        val source = FakePort()
        val controller = FileTransferController(source)
        val updates = mutableListOf<FileTransferStage>()
        val subscription = controller.observe { updates += it.stage }

        source.emit(FileTransferStage.TRANSFERRING)
        assertEquals(FileTransferStage.TRANSFERRING, controller.state().stage)
        assertEquals(
            listOf(FileTransferStage.IDLE, FileTransferStage.TRANSFERRING),
            updates,
        )

        subscription.close()
        source.emit(FileTransferStage.COMPLETED)
        assertEquals(2, updates.size)
        controller.shutdown()
        assertTrue(source.unsubscribed)
    }

    @Test
    fun transfer_progress_never_exceeds_declared_size() {
        val state = FileTransferState(
            available = true,
            totalBytes = 10uL,
            transferredBytes = 11uL,
            stage = FileTransferStage.TRANSFERRING,
        )
        assertEquals("10 / 10 bytes · 100%", progressLabel(state))
    }

    private class FakePort : FileTransferPort {
        private var listener: ((FileTransferState) -> Unit)? = null
        var unsubscribed = false
            private set

        override fun observeFileTransfer(listener: (FileTransferState) -> Unit): AutoCloseable {
            this.listener = listener
            listener(FileTransferState.initial(true))
            return AutoCloseable {
                this.listener = null
                unsubscribed = true
            }
        }

        fun emit(stage: FileTransferStage) {
            listener?.invoke(FileTransferState(available = true, stage = stage))
        }
    }
}
