package dev.crosslab.android.features.appearance

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class ThemeParserTest {
    @Test
    fun parsesCanonicalThemeFixture() {
        val theme = ThemeParser.parse(resource("fixtures/theme-v1-valid.json"))

        assertEquals(1, theme.schemaVersion)
        assertEquals("fixture-light", theme.id)
        assertEquals(ThemeAppearance.LIGHT, theme.appearance)
        assertEquals(0.0, theme.radius.none, 0.0)
        assertEquals(0.2, theme.colors.selection.alpha ?: error("selection alpha"), 0.0)
        assertEquals(32.0, theme.metrics.rowHeight, 0.0)
    }

    @Test
    fun missingRequiredThemeFieldFailsClosed() {
        assertThrows(ThemeParseException::class.java) {
            ThemeParser.parse(resource("fixtures/theme-v1-invalid.json"))
        }
    }

    @Test
    fun unsupportedSchemaVersionFailsClosed() {
        val unsupported = resource("fixtures/theme-v1-valid.json")
            .replace("\"schema_version\": 1", "\"schema_version\": 2")

        assertThrows(UnsupportedThemeSchemaException::class.java) {
            ThemeParser.parse(unsupported)
        }
    }

    private fun resource(path: String): String =
        checkNotNull(javaClass.classLoader?.getResource(path)) { "missing test resource: $path" }
            .readText()
}
