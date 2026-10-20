package dev.crosslab.android.features.filetransfer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FileTransferControllerTest {
    @Test
    fun throwingObserverCannotInterruptTransferStateDelivery() {
        val source = FakePort()
        val controller = FileTransferController(source)
        val bad = controller.observe { throw IllegalStateException("stale Compose screen") }
        val stages = mutableListOf<FileTransferStage>()
        val good = controller.observe { stages += it.stage }

        source.emit(FileTransferStage.TRANSFERRING)
        source.emit(FileTransferStage.COMPLETED)

        assertEquals(FileTransferStage.COMPLETED, controller.state().stage)
        assertEquals(
            listOf(FileTransferStage.IDLE, FileTransferStage.TRANSFERRING, FileTransferStage.COMPLETED),
            stages,
        )
        bad.close()
        good.close()
        controller.shutdown()
    }

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


    @Test
    fun native_action_exception_fails_closed_and_reports_recovery_without_payload() {
        val source = FaultyPort()
        val controller = FileTransferController(source)
        val states = mutableListOf<FileTransferState>()
        val observer = controller.observe { states += it }
        source.failAction = true
        assertFalse(controller.retry())
        val failed = controller.state()
        assertFalse(failed.available)
        assertEquals(FileTransferStage.FAILED, failed.stage)
        assertEquals(FileTransferFailure.UNAVAILABLE, failed.failure)
        assertFalse(failed.canCancel)
        assertFalse(failed.canRetry)
        assertNull(failed.pendingRequestId)
        assertNull(failed.displayName)
        assertTrue(states.last() === failed)
        assertFalse(controller.resume("opaque-request"))
        assertFalse(controller.decline())
        assertFalse(controller.cancel())
        observer.close()
        controller.shutdown()
    }

    @Test
    fun native_publication_can_restore_service_after_transient_action_exception() {
        val source = FaultyPort()
        val controller = FileTransferController(source)
        source.failAction = true
        assertFalse(controller.cancel())
        source.failAction = false
        source.emit(FileTransferState(available = true, stage = FileTransferStage.IDLE))
        assertTrue(controller.state().available)
        assertEquals(FileTransferStage.IDLE, controller.state().stage)
        assertTrue(controller.retry())
        controller.shutdown()
    }

    @Test
    fun failed_action_return_does_not_invent_a_native_state_transition() {
        val source = FaultyPort().apply { returnFalse = true }
        val controller = FileTransferController(source)
        assertFalse(controller.cancel())
        assertEquals(FileTransferStage.IDLE, controller.state().stage)
        assertTrue(controller.state().available)
        controller.shutdown()
    }

    @Test
    fun simultaneous_native_publication_must_not_be_overwritten_by_a_stale_exception() {
        val source = FaultyPort()
        val controller = FileTransferController(source)
        source.emitOnAction = FileTransferState(available = true, stage = FileTransferStage.TRANSFERRING)
        source.failAction = true
        assertFalse(controller.retry())
        assertEquals(FileTransferStage.TRANSFERRING, controller.state().stage)
        assertTrue(controller.state().available)
        controller.shutdown()
    }

    @Test
    fun late_native_updates_and_actions_are_ignored_after_shutdown() {
        val source = FaultyPort().apply { failClose = true }
        val controller = FileTransferController(source)
        val updates = mutableListOf<FileTransferStage>()
        val sub = controller.observe { updates += it.stage }
        controller.shutdown()
        controller.shutdown()
        assertEquals(FileTransferStage.IDLE, controller.state().stage)
        assertFalse(controller.state().available)
        source.emit(FileTransferState(available = true, stage = FileTransferStage.COMPLETED))
        assertFalse(controller.retry())
        assertEquals(FileTransferStage.IDLE, controller.state().stage)
        assertEquals(listOf(FileTransferStage.IDLE, FileTransferStage.IDLE), updates)
        sub.close()
    }

    @Test
    fun native_observation_start_exception_returns_unavailable_without_throwing() {
        val source = FaultyPort().apply { failObserve = true }
        val controller = FileTransferController(source)
        assertEquals(FileTransferStage.FAILED, controller.state().stage)
        assertEquals(FileTransferFailure.UNAVAILABLE, controller.state().failure)
        controller.shutdown()
    }

    private class FaultyPort : FileTransferPort {
        var failAction = false
        var returnFalse = false
        var failObserve = false
        var failClose = false
        var emitOnAction: FileTransferState? = null
        private var listener: ((FileTransferState) -> Unit)? = null

        override val fileTransferAvailable = true

        override fun observeFileTransfer(listener: (FileTransferState) -> Unit): AutoCloseable {
            if (failObserve) throw IllegalStateException("native subscription unavailable")
            this.listener = listener
            listener(FileTransferState.initial(true))
            return AutoCloseable {
                if (failClose) throw IllegalStateException("teardown fault")
                this.listener = null
            }
        }

        fun emit(state: FileTransferState) {
            listener?.invoke(state)
        }

        override fun retryFile() = action()
        override fun declineFile() = action()
        override fun cancelFile() = action()
        override fun resumeFile(requestId: String) = action()

        private fun action(): Boolean {
            emitOnAction?.let { emit(it) }
            if (failAction) throw IllegalStateException("native file adapter failed")
            return !returnFalse
        }
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
