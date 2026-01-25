package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.ui.Messages

/**
 * Action to generate CCGO project documentation.
 */
class CcgoDocAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        if (!CcgoProjectService.getInstance(project).hasCcgoToml()) {
            Messages.showErrorDialog(project, "No CCGO.toml found in project", "CCGO Documentation")
            return
        }

        val options = arrayOf("Generate Only", "Generate and Open")
        val choice = Messages.showDialog(
            project,
            "Select documentation option:",
            "Generate Documentation",
            options,
            1,  // Default to "Generate and Open"
            Messages.getQuestionIcon()
        )

        when (choice) {
            0 -> CcgoCommandExecutor.execute(project, listOf("doc"))
            1 -> CcgoCommandExecutor.execute(project, listOf("doc", "--open"))
        }
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }
}
