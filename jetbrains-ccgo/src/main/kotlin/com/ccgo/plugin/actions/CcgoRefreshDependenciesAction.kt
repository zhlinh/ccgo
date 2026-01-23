package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.ccgo.plugin.toolwindow.DependencyTreePanel
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.wm.ToolWindowManager

/**
 * Action to refresh the dependency tree view.
 */
class CcgoRefreshDependenciesAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return

        // Get the tool window and refresh its content
        val toolWindow = ToolWindowManager.getInstance(project)
            .getToolWindow("CCGO Dependencies") ?: return

        val content = toolWindow.contentManager.getContent(0) ?: return
        val panel = content.component as? DependencyTreePanel ?: return

        panel.loadDependencies()
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        e.presentation.isEnabled = project != null &&
                CcgoProjectService.getInstance(project).hasCcgoToml()
    }
}
