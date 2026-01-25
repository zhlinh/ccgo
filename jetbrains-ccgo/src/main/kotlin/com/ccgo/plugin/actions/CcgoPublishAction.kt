package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.ui.Messages

/**
 * Action to publish CCGO package to various repositories.
 */
class CcgoPublishAction : AnAction() {

    private val publishTargets = arrayOf(
        "android" to "Publish to Maven repository",
        "ios" to "Publish iOS framework",
        "macos" to "Publish macOS framework",
        "ohos" to "Publish to OHPM repository",
        "maven" to "Publish to Maven",
        "cocoapods" to "Publish to CocoaPods",
        "spm" to "Publish to Swift Package Manager",
        "index" to "Publish to package index"
    )

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        if (!CcgoProjectService.getInstance(project).hasCcgoToml()) {
            Messages.showErrorDialog(project, "No CCGO.toml found in project", "CCGO Publish")
            return
        }

        val targetLabels = publishTargets.map { "${it.first} - ${it.second}" }.toTypedArray()
        val choice = Messages.showDialog(
            project,
            "Select publish target:",
            "Publish Package",
            targetLabels,
            0,
            Messages.getQuestionIcon()
        )

        if (choice >= 0 && choice < publishTargets.size) {
            val target = publishTargets[choice].first
            CcgoCommandExecutor.execute(project, listOf("publish", target))
        }
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }
}
