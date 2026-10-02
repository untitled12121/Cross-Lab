package dev.crosslab.android.features.notifications

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class NotificationDisplayBoundsTest {
    @Test
    fun stripsControlsAndBoundsUtf8WithoutSplittingCharacters() {
        assertEquals("ab", boundedDisplay("a\u0000b", 3))
        assertEquals("c", boundedDisplay("\ud83d\udd12c", 4))
        assertEquals("é", boundedDisplay("éa", 2))
        assertEquals("", boundedDisplay("é", 1))
        assertNull(boundedDisplay(null, 96))
    }
}
