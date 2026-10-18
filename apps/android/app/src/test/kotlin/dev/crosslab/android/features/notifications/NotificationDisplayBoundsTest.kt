package dev.crosslab.android.features.notifications

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import android.app.Notification
import org.junit.Test

class NotificationDisplayBoundsTest {
    @Test
    fun excludesSystemCallAndServiceNotificationsBeforeCapture() {
        assertTrue(excludedNotificationSource("android", null))
        assertTrue(excludedNotificationSource("com.android.systemui", null))
        assertTrue(excludedNotificationSource("example.app", Notification.CATEGORY_CALL))
        assertTrue(excludedNotificationSource("example.app", Notification.CATEGORY_SYSTEM))
        assertTrue(excludedNotificationSource("example.app", Notification.CATEGORY_SERVICE))
        assertFalse(excludedNotificationSource("example.app", Notification.CATEGORY_MESSAGE))
        assertFalse(excludedNotificationSource("example.app", null))
    }

    @Test
    fun stripsControlsAndBoundsUtf8WithoutSplittingCharacters() {
        assertEquals("ab", boundedDisplay("a\u0000b", 3))
        assertEquals("🔒", boundedDisplay("\ud83d\udd12c", 4))
        assertEquals("🔒c", boundedDisplay("\ud83d\udd12c", 5))
        assertEquals("", boundedDisplay("\ud83d\udd12c", 3))
        assertEquals("ac", boundedDisplay("a\uD800c\uDC00", 2))
        assertEquals("é", boundedDisplay("éa", 2))
        assertEquals("", boundedDisplay("é", 1))
        assertNull(boundedDisplay(null, 96))
    }
}
