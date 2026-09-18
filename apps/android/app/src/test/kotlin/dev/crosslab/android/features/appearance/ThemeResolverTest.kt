package dev.crosslab.android.features.appearance

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class ThemeResolverTest {
    @Test
    fun systemSelectionUsesInjectedAppearance() {
        assertEquals(
            ThemeId.AYU_LIGHT,
            ThemeResolver.resolve(ThemeSelection.SYSTEM, SystemAppearance.LIGHT),
        )
        assertEquals(
            ThemeId.DARKMATTER,
            ThemeResolver.resolve(ThemeSelection.SYSTEM, SystemAppearance.DARK),
        )
    }

    @Test
    fun explicitSelectionIgnoresSystemAppearance() {
        assertEquals(
            ThemeId.AYU_LIGHT,
            ThemeResolver.resolve(ThemeSelection.AYU_LIGHT, SystemAppearance.DARK),
        )
        assertEquals(
            ThemeId.DARKMATTER,
            ThemeResolver.resolve(ThemeSelection.DARKMATTER, SystemAppearance.LIGHT),
        )
    }

    @Test
    fun missingDarkmatterRemainsExplicitlyUnavailable() {
        assertThrows(ThemeUnavailableException::class.java) {
            ThemeResolver.assetName(ThemeId.DARKMATTER)
        }
    }
}
