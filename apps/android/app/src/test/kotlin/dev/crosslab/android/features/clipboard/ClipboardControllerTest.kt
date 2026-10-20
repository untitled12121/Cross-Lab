package dev.crosslab.android.features.clipboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ClipboardControllerTest {
    @Test
    fun throwingObserverCannotAbortClipboardOperationOrOtherSubscribers() {
        val port = FakeClipboardPort()
        val controller = ClipboardController(port)
        val seen = mutableListOf<ClipboardResult?>()
        val bad = controller.observe { throw IllegalStateException("UI observer went away") }
        val good = controller.observe { seen += it.result }

        controller.send()
        port.complete(ClipboardResult.SUCCESS)

        assertEquals(1, port.sendCalls)
        assertEquals(ClipboardResult.SUCCESS, controller.state().result)
        assertEquals(ClipboardResult.SUCCESS, seen.last())
        good.close()
        bad.close()
    }

    @Test
    fun sendPublishesBusyThenSuccessWithoutPayloadState() {
        val port = FakeClipboardPort()
        val controller = ClipboardController(port)

        controller.send()

        assertTrue(controller.state().busy)
        assertEquals(ClipboardAction.SEND, controller.state().action)
        assertNull(controller.state().result)

        port.complete(ClipboardResult.SUCCESS)

        assertFalse(controller.state().busy)
        assertEquals(ClipboardResult.SUCCESS, controller.state().result)
    }

    @Test
    fun overlappingActionsAreBoundedToOneOperation() {
        val port = FakeClipboardPort()
        val controller = ClipboardController(port)

        controller.send()
        controller.fetch()

        assertEquals(1, port.sendCalls)
        assertEquals(0, port.fetchCalls)

        port.complete(ClipboardResult.CANCELLED)
        controller.fetch()

        assertEquals(1, port.fetchCalls)
        assertEquals(ClipboardAction.FETCH, controller.state().action)
    }

    @Test
    fun lateCallbackFromPriorSendCannotCompleteNewSend() {
        val port = FakeClipboardPort()
        val controller = ClipboardController(port)

        controller.send()
        val stale = port.capturedCompletion()
        port.complete(ClipboardResult.SUCCESS)
        controller.send()

        stale(ClipboardResult.FAILED)
        assertTrue(controller.state().busy)
        assertNull(controller.state().result)

        port.complete(ClipboardResult.SUCCESS)
        assertFalse(controller.state().busy)
        assertEquals(ClipboardResult.SUCCESS, controller.state().result)
        assertEquals(2, port.sendCalls)
    }

    @Test
    fun throwingPortReportsFailureAndReleasesBusyState() {
        val controller = ClipboardController(object : ClipboardPort {
            override val clipboardAvailable = true
            override fun sendClipboard(onComplete: (ClipboardResult) -> Unit): Boolean {
                throw IllegalStateException("simulated native port failure")
            }
        })
        controller.send()
        assertFalse(controller.state().busy)
        assertEquals(ClipboardResult.FAILED, controller.state().result)
    }

    @Test
    fun shutdownInvalidatesPendingCallbacksAndForbidsNewWork() {
        val port = FakeClipboardPort()
        val controller = ClipboardController(port)
        controller.fetch()
        val stale = port.capturedCompletion()
        controller.shutdown()

        stale(ClipboardResult.SUCCESS)
        controller.fetch()
        controller.send()

        assertFalse(controller.state().busy)
        assertFalse(controller.state().available)
        assertNull(controller.state().result)
        assertEquals(1, port.fetchCalls)
        assertEquals(0, port.sendCalls)
    }

    @Test
    fun unavailablePortDoesNotStartClipboardWork() {
        val controller = ClipboardController(UnavailableClipboardPort)

        controller.send()

        assertFalse(controller.state().available)
        assertFalse(controller.state().busy)
        assertNull(controller.state().action)
        assertNull(controller.state().result)
    }

    private class FakeClipboardPort : ClipboardPort {
        override val clipboardAvailable = true
        var sendCalls = 0
        var fetchCalls = 0
        private var completion: ((ClipboardResult) -> Unit)? = null

        override fun sendClipboard(onComplete: (ClipboardResult) -> Unit): Boolean {
            sendCalls += 1
            completion = onComplete
            return true
        }

        override fun fetchClipboard(onComplete: (ClipboardResult) -> Unit): Boolean {
            fetchCalls += 1
            completion = onComplete
            return true
        }

        fun capturedCompletion(): (ClipboardResult) -> Unit = checkNotNull(completion)

        fun complete(result: ClipboardResult) {
            val callback = checkNotNull(completion)
            completion = null
            callback(result)
        }
    }
}
