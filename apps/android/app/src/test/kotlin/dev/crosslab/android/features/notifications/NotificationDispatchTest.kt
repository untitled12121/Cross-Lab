package dev.crosslab.android.features.notifications

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NotificationDispatchTest {
    private class FakePublisher : NotificationPublisher {
        var capture = false
        var content = false
        var failCapture = false
        var failContent = false
        var failDelivery = false
        var delivered: NotificationDelivery? = null
        var removedKey: String? = null

        override fun canCapture(): Boolean {
            if (failCapture) throw IllegalStateException("capture unavailable")
            return capture
        }

        override fun contentAllowed(): Boolean {
            if (failContent) throw IllegalStateException("consent store unavailable")
            return content
        }

        override fun posted(
            key: String,
            appLabel: String,
            title: String?,
            preview: String?,
            protected: Boolean,
        ): Boolean {
            if (failDelivery) throw IllegalStateException("native send unavailable")
            delivered = NotificationDelivery(key, appLabel, title, preview, protected)
            return true
        }

        override fun removed(key: String): Boolean {
            if (failDelivery) throw IllegalStateException("native removal unavailable")
            removedKey = key
            return true
        }

        override fun disableNotifications() = Unit

        override fun observeSubscription(listener: (Boolean) -> Unit): AutoCloseable =
            AutoCloseable {}
    }

    private fun payload() =
        NotificationDelivery("opaque-local-key", "App", "private title", "private preview", false)

    @Test
    fun blockedCaptureDoesNotInspectNotificationContentsOrKeys() {
        val publisher = FakePublisher()
        val dispatch = NotificationDispatch(publisher)
        var inspected = 0
        assertFalse(dispatch.posted { inspected++; payload() })
        assertFalse(dispatch.removed { inspected++; "opaque-local-key" })
        assertEquals(0, inspected)

        publisher.capture = true
        publisher.failCapture = true
        assertFalse(dispatch.posted { inspected++; payload() })
        assertFalse(dispatch.removed { inspected++; "opaque-local-key" })
        assertEquals(0, inspected)
    }

    @Test
    fun failedContentConsentPreventsMetadataDelivery() {
        val publisher = FakePublisher().apply {
            capture = true
            failContent = true
        }
        var inspected = false
        assertFalse(NotificationDispatch(publisher).posted {
            inspected = true
            payload()
        })
        assertFalse(inspected)
        assertNull(publisher.delivered)
    }

    @Test
    fun redactedAndProtectedMessagesNeverForwardPreparedPrivateText() {
        val publisher = FakePublisher().apply { capture = true }
        assertTrue(NotificationDispatch(publisher).posted { payload() })
        assertEquals("App", publisher.delivered?.appLabel)
        assertNull(publisher.delivered?.title)
        assertNull(publisher.delivered?.preview)

        publisher.content = true
        assertTrue(NotificationDispatch(publisher).posted {
            payload().copy(isProtected = true)
        })
        assertNull(publisher.delivered?.title)
        assertNull(publisher.delivered?.preview)

        assertTrue(NotificationDispatch(publisher).posted { payload() })
        assertEquals("private title", publisher.delivered?.title)
    }

    @Test
    fun nativeDeliveryAndExtractionFaultsAreContained() {
        val publisher = FakePublisher().apply { capture = true; content = true }
        val dispatch = NotificationDispatch(publisher)
        assertFalse(dispatch.posted { throw IllegalStateException("broken OS extras") })
        publisher.failDelivery = true
        assertFalse(dispatch.posted { payload() })
        assertFalse(dispatch.removed { "opaque-local-key" })
        assertNull(publisher.delivered)
        assertNull(publisher.removedKey)
        assertFalse(dispatch.removed { throw IllegalStateException("stale OS key") })
    }

    @Test
    fun removalOnlyUsesAnAuthorizedCaptureAndSuppliedKey() {
        val publisher = FakePublisher().apply { capture = true }
        val dispatch = NotificationDispatch(publisher)
        assertTrue(dispatch.removed { "opaque-local-key" })
        assertEquals("opaque-local-key", publisher.removedKey)
        assertFalse(dispatch.removed { null })
    }
}
