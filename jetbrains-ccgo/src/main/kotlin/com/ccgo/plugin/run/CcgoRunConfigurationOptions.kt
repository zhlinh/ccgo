package com.ccgo.plugin.run

import com.intellij.execution.configurations.RunConfigurationOptions
import com.intellij.openapi.components.StoredProperty

/**
 * Options (state) for CCGO run configuration.
 */
class CcgoRunConfigurationOptions : RunConfigurationOptions() {

    private val commandProperty: StoredProperty<String?> = string("build")
        .provideDelegate(this, "command")

    private val platformProperty: StoredProperty<String?> = string("macos")
        .provideDelegate(this, "platform")

    private val releaseProperty: StoredProperty<Boolean> = property(false)
        .provideDelegate(this, "release")

    private val architecturesProperty: StoredProperty<String?> = string("")
        .provideDelegate(this, "architectures")

    private val ideProjectProperty: StoredProperty<Boolean> = property(false)
        .provideDelegate(this, "ideProject")

    private val additionalArgsProperty: StoredProperty<String?> = string("")
        .provideDelegate(this, "additionalArgs")

    var command: String
        get() = commandProperty.getValue(this) ?: "build"
        set(value) = commandProperty.setValue(this, value)

    var platform: String
        get() = platformProperty.getValue(this) ?: "macos"
        set(value) = platformProperty.setValue(this, value)

    var release: Boolean
        get() = releaseProperty.getValue(this)
        set(value) = releaseProperty.setValue(this, value)

    var architectures: String
        get() = architecturesProperty.getValue(this) ?: ""
        set(value) = architecturesProperty.setValue(this, value)

    var ideProject: Boolean
        get() = ideProjectProperty.getValue(this)
        set(value) = ideProjectProperty.setValue(this, value)

    var additionalArgs: String
        get() = additionalArgsProperty.getValue(this) ?: ""
        set(value) = additionalArgsProperty.setValue(this, value)
}
