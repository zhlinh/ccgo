package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.ui.Messages

/**
 * Action to clean CCGO build artifacts.
 */
class CcgoCleanAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        if (!CcgoProjectService.getInstance(project).hasCcgoToml()) {
            Messages.showErrorDialog(project, "No CCGO.toml found in project", "CCGO Clean")
            return
        }

        val result = Messages.showYesNoDialog(
            project,
            "This will remove all build artifacts. Continue?",
            "CCGO Clean",
            Messages.getQuestionIcon()
        )

        if (result == Messages.YES) {
            CcgoCommandExecutor.execute(project, listOf("clean", "-y"))
        }
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }
}
