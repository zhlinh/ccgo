package com.ccgo.plugin.settings

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.util.xmlb.XmlSerializerUtil

/**
 * Application-level settings for CCGO plugin.
 * Settings are persisted in the IDE configuration directory.
 */
@State(
    name = "com.ccgo.plugin.settings.CcgoSettings",
    storages = [Storage("CcgoSettings.xml")]
)
class CcgoSettings : PersistentStateComponent<CcgoSettings.State> {

    /**
     * Persistent state data class.
     */
    data class State(
        var executablePath: String = "ccgo",
        var defaultPlatform: String = "",
        var autoRefreshDependencies: Boolean = true,
        var showNotifications: Boolean = true,
        var buildInTerminal: Boolean = true
    )

    private var state = State()

    override fun getState(): State = state

    override fun loadState(state: State) {
        XmlSerializerUtil.copyBean(state, this.state)
    }

    var executablePath: String
        get() = state.executablePath
        set(value) { state.executablePath = value }

    var defaultPlatform: String
        get() = state.defaultPlatform
        set(value) { state.defaultPlatform = value }

    var autoRefreshDependencies: Boolean
        get() = state.autoRefreshDependencies
        set(value) { state.autoRefreshDependencies = value }

    var showNotifications: Boolean
        get() = state.showNotifications
        set(value) { state.showNotifications = value }

    var buildInTerminal: Boolean
        get() = state.buildInTerminal
        set(value) { state.buildInTerminal = value }

    companion object {
        /**
         * List of supported platforms for CCGO builds.
         */
        val PLATFORMS = listOf(
            "" to "Auto-detect",
            "android" to "Android",
            "ios" to "iOS",
            "macos" to "macOS",
            "linux" to "Linux",
            "windows" to "Windows",
            "ohos" to "OpenHarmony",
            "kmp" to "Kotlin Multiplatform"
        )

        @JvmStatic
        fun getInstance(): CcgoSettings {
            return ApplicationManager.getApplication().getService(CcgoSettings::class.java)
        }
    }
}
