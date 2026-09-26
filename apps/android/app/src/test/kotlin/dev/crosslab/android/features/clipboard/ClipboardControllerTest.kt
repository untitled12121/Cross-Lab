package dev.crosslab.android.features.clipboard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ClipboardControllerTest {
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

        fun complete(result: ClipboardResult) {
            val callback = checkNotNull(completion)
            completion = null
            callback(result)
        }
    }
}
