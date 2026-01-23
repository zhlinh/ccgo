package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.ccgo.plugin.settings.CcgoSettings
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.DialogWrapper
import com.intellij.openapi.ui.Messages
import com.intellij.ui.dsl.builder.*
import javax.swing.JComponent

/**
 * Action to build a CCGO project.
 * Shows a dialog to select platform and build options.
 */
class CcgoBuildAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        if (!CcgoProjectService.getInstance(project).hasCcgoToml()) {
            Messages.showErrorDialog(project, "No CCGO.toml found in project", "CCGO Build")
            return
        }

        val dialog = BuildDialog(project)
        if (dialog.showAndGet()) {
            val command = buildCommand(dialog)
            CcgoCommandExecutor.execute(project, command)
        }
    }

    private fun buildCommand(dialog: BuildDialog): List<String> {
        val args = mutableListOf("build", dialog.platform)

        if (dialog.release) {
            args.add("--release")
        }

        if (dialog.ideProject) {
            args.add("--ide-project")
        }

        if (dialog.architectures.isNotBlank()) {
            args.add("--arch")
            args.add(dialog.architectures)
        }

        return args
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }

    /**
     * Dialog for build configuration.
     */
    private class BuildDialog(project: Project) : DialogWrapper(project) {
        var platform: String = CcgoSettings.getInstance().defaultPlatform.ifEmpty {
            CcgoProjectService.getInstance(project).detectCurrentPlatform()
        }
        var release: Boolean = false
        var ideProject: Boolean = false
        var architectures: String = ""

        init {
            title = "Build CCGO Project"
            init()
        }

        override fun createCenterPanel(): JComponent = panel {
            row("Platform:") {
                comboBox(PLATFORMS)
                    .bindItem(
                        getter = { PLATFORMS.find { it.first == platform } ?: PLATFORMS[0] },
                        setter = { platform = it?.first ?: "" }
                    )
            }

            row("Architectures:") {
                textField()
                    .bindText(::architectures)
                    .comment("Comma-separated list (e.g., arm64-v8a,armeabi-v7a)")
            }

            row {
                checkBox("Release build")
                    .bindSelected(::release)
            }

            row {
                checkBox("Generate IDE project")
                    .bindSelected(::ideProject)
            }
        }

        companion object {
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
}
