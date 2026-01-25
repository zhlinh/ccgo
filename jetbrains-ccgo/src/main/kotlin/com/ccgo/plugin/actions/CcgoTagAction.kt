package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.ui.Messages

/**
 * Action to create a git version tag from CCGO.toml version.
 */
class CcgoTagAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        if (!CcgoProjectService.getInstance(project).hasCcgoToml()) {
            Messages.showErrorDialog(project, "No CCGO.toml found in project", "CCGO Tag")
            return
        }

        val version = Messages.showInputDialog(
            project,
            "Enter version tag (leave empty to use version from CCGO.toml):",
            "Create Version Tag",
            Messages.getQuestionIcon(),
            "",
            null
        )

        // If user canceled, version will be null
        if (version != null) {
            if (version.isNotEmpty()) {
                CcgoCommandExecutor.execute(project, listOf("tag", version))
            } else {
                CcgoCommandExecutor.execute(project, listOf("tag"))
            }
        }
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }
}
