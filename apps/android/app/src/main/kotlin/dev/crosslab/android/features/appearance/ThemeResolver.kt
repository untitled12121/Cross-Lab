package dev.crosslab.android.features.appearance

object ThemeResolver {
    fun resolve(selection: ThemeSelection, system: SystemAppearance): ThemeId =
        when (selection) {
            ThemeSelection.AYU_LIGHT -> ThemeId.AYU_LIGHT
            ThemeSelection.DARKMATTER -> ThemeId.DARKMATTER
            ThemeSelection.SYSTEM ->
                when (system) {
                    SystemAppearance.LIGHT -> ThemeId.AYU_LIGHT
                    SystemAppearance.DARK -> ThemeId.DARKMATTER
                }
        }

    fun assetName(themeId: ThemeId): String =
        when (themeId) {
            ThemeId.AYU_LIGHT -> "ayu-light.json"
            ThemeId.DARKMATTER -> throw ThemeUnavailableException(themeId)
        }
}
