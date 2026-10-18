package dev.crosslab.android.features.networking

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LanNetworkTrackerTest {
    @Test
    fun overlappingLanTransportsOnlyReportRealAvailabilityTransitions() {
        val tracker = LanNetworkTracker<String>()

        assertFalse(tracker.disappeared("unknown"))
        assertTrue(tracker.appeared("wifi"))
        assertFalse(tracker.appeared("wifi"))
        assertFalse(tracker.appeared("ethernet"))
        assertFalse(tracker.disappeared("wifi"))
        assertTrue(tracker.disappeared("ethernet"))
        assertFalse(tracker.disappeared("ethernet"))
        assertTrue(tracker.appeared("new-wifi"))
    }
}
