package com.ccgo.plugin.toolwindow

import com.ccgo.plugin.CcgoIcons
import com.ccgo.plugin.CcgoProjectService
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.content.ContentFactory

/**
 * Factory for creating the CCGO Dependencies tool window.
 */
class DependencyTreeToolWindowFactory : ToolWindowFactory, DumbAware {

    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = DependencyTreePanel(project)
        val content = ContentFactory.getInstance().createContent(panel, "", false)
        toolWindow.contentManager.addContent(content)
    }

    override fun shouldBeAvailable(project: Project): Boolean {
        return CcgoProjectService.getInstance(project).hasCcgoToml()
    }

    override fun init(toolWindow: ToolWindow) {
        toolWindow.setIcon(CcgoIcons.CCGO_13)
    }
}
