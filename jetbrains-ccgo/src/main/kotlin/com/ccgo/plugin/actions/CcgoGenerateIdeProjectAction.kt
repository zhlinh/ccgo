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
 * Action to generate platform-specific IDE projects.
 */
class CcgoGenerateIdeProjectAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        if (!CcgoProjectService.getInstance(project).hasCcgoToml()) {
            Messages.showErrorDialog(project, "No CCGO.toml found in project", "Generate IDE Project")
            return
        }

        val dialog = GenerateIdeProjectDialog(project)
        if (dialog.showAndGet()) {
            CcgoCommandExecutor.execute(project, listOf("build", dialog.platform, "--ide-project"))
        }
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }

    /**
     * Dialog for selecting platform for IDE project generation.
     */
    private class GenerateIdeProjectDialog(project: Project) : DialogWrapper(project) {
        var platform: String = CcgoSettings.getInstance().defaultPlatform.ifEmpty {
            CcgoProjectService.getInstance(project).detectCurrentPlatform()
        }

        init {
            title = "Generate IDE Project"
            init()
        }

        override fun createCenterPanel(): JComponent = panel {
            row("Platform:") {
                comboBox(PLATFORMS)
                    .bindItem(
                        getter = { PLATFORMS.find { it.first == platform } ?: PLATFORMS[0] },
                        setter = { platform = it?.first ?: "" }
                    )
                    .comment("Select the target platform for IDE project generation")
            }

            row {
                label("This will generate:")
            }

            row {
                label("  • iOS/macOS: Xcode project (.xcodeproj)")
            }

            row {
                label("  • Windows: Visual Studio solution (.sln)")
            }

            row {
                label("  • Linux: CodeLite workspace + compile_commands.json")
            }

            row {
                label("  • Android: Android Studio project (build.gradle)")
            }
        }

        companion object {
            val PLATFORMS = listOf(
                "macos" to "macOS (Xcode)",
                "ios" to "iOS (Xcode)",
                "windows" to "Windows (Visual Studio)",
                "linux" to "Linux (CodeLite)",
                "android" to "Android (Android Studio)"
            )
        }
    }
}
