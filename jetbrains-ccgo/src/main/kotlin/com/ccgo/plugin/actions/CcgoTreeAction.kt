package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.ui.Messages

/**
 * Action to show CCGO dependency tree in terminal.
 */
class CcgoTreeAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        if (!CcgoProjectService.getInstance(project).hasCcgoToml()) {
            Messages.showErrorDialog(project, "No CCGO.toml found in project", "CCGO Dependency Tree")
            return
        }

        CcgoCommandExecutor.execute(project, listOf("tree"))
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }
}
