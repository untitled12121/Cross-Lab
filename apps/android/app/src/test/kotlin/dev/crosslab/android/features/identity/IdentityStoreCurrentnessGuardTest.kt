package dev.crosslab.android.features.identity

import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class IdentityStoreCurrentnessGuardTest {
    @Test
    fun deletedCommittedIdentityCannotBeMistakenForFreshOwnerBootstrap() {
        val committedKey = "crosslab.identity.anchor.v1.7"

        // Simulates local snapshot deletion/rollback while its non-exportable
        // Android Keystore currentness anchor remains. The old implementation
        // returned null and allowed the owner-bootstrap path to run.
        assertThrows(IdentityStoreUnavailable::class.java) {
            identityStateFilePresent(
                stateExists = false,
                currentnessKeys = listOf(committedKey),
            )
        }

        // An untouched installation may still create its first identity.
        assertFalse(identityStateFilePresent(false, emptyList()))
        assertTrue(identityStateFilePresent(true, listOf(committedKey)))

        // After a state file reappears, it can only be admitted with the exact
        // expected revision's sole currentness key.
        verifyIdentityAnchorSet(committedKey, listOf(committedKey))
        assertThrows(IdentityStoreUnavailable::class.java) {
            verifyIdentityAnchorSet(committedKey, listOf("crosslab.identity.anchor.v1.6"))
        }
        assertThrows(IdentityStoreUnavailable::class.java) {
            verifyIdentityAnchorSet(
                committedKey,
                listOf(committedKey, "crosslab.identity.anchor.v1.6"),
            )
        }
        assertThrows(IdentityStoreUnavailable::class.java) {
            verifyIdentityAnchorSet(committedKey, emptyList())
        }
    }
}
