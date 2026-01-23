package com.ccgo.plugin.run

import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import com.intellij.ui.dsl.builder.*
import javax.swing.JComponent

/**
 * Editor UI for CCGO run configurations.
 */
class CcgoRunConfigurationEditor(private val project: Project) : SettingsEditor<CcgoRunConfiguration>() {

    private var command: String = "build"
    private var platform: String = "macos"
    private var release: Boolean = false
    private var architectures: String = ""
    private var ideProject: Boolean = false
    private var additionalArgs: String = ""

    override fun resetEditorFrom(configuration: CcgoRunConfiguration) {
        command = configuration.command
        platform = configuration.platform
        release = configuration.release
        architectures = configuration.architectures
        ideProject = configuration.ideProject
        additionalArgs = configuration.additionalArgs
    }

    override fun applyEditorTo(configuration: CcgoRunConfiguration) {
        configuration.command = command
        configuration.platform = platform
        configuration.release = release
        configuration.architectures = architectures
        configuration.ideProject = ideProject
        configuration.additionalArgs = additionalArgs
    }

    override fun createEditor(): JComponent = panel {
        row("Command:") {
            comboBox(COMMANDS)
                .bindItem(
                    getter = { COMMANDS.find { it.first == command } ?: COMMANDS[0] },
                    setter = { command = it?.first ?: "build" }
                )
                .comment("The CCGO command to execute")
        }

        row("Platform:") {
            comboBox(PLATFORMS)
                .bindItem(
                    getter = { PLATFORMS.find { it.first == platform } ?: PLATFORMS[0] },
                    setter = { platform = it?.first ?: "macos" }
                )
                .comment("Target platform for build commands")
        }

        row("Architectures:") {
            textField()
                .bindText(::architectures)
                .columns(COLUMNS_LARGE)
                .comment("Comma-separated list (e.g., arm64-v8a,armeabi-v7a)")
        }

        row {
            checkBox("Release build")
                .bindSelected(::release)
        }

        row {
            checkBox("Generate IDE project")
                .bindSelected(::ideProject)
                .comment("Generate platform-specific IDE project files")
        }

        row("Additional arguments:") {
            textField()
                .bindText(::additionalArgs)
                .columns(COLUMNS_LARGE)
                .comment("Additional command line arguments")
        }
    }

    companion object {
        val COMMANDS = listOf(
            "build" to "Build",
            "test" to "Test",
            "bench" to "Benchmark",
            "clean" to "Clean",
            "install" to "Install Dependencies",
            "doc" to "Generate Documentation"
        )

        val PLATFORMS = listOf(
            "android" to "Android",
            "ios" to "iOS",
            "macos" to "macOS",
            "linux" to "Linux",
            "windows" to "Windows",
            "ohos" to "OpenHarmony",
            "kmp" to "Kotlin Multiplatform"
        )
    }
}
