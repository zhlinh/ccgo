package com.ccgo.plugin.toolwindow

import com.ccgo.plugin.CcgoIcons
import com.ccgo.plugin.CcgoProjectService
import com.ccgo.plugin.settings.CcgoSettings
import com.google.gson.Gson
import com.google.gson.JsonArray
import com.google.gson.JsonObject
import com.intellij.icons.AllIcons
import com.intellij.openapi.actionSystem.*
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.project.Project
import com.intellij.ui.JBColor
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.treeStructure.Tree
import com.intellij.util.ui.JBUI
import java.awt.BorderLayout
import java.io.BufferedReader
import java.io.InputStreamReader
import javax.swing.JPanel
import javax.swing.tree.DefaultMutableTreeNode
import javax.swing.tree.DefaultTreeModel

/**
 * Panel that displays the dependency tree for a CCGO project.
 */
class DependencyTreePanel(private val project: Project) : JPanel(BorderLayout()) {

    private val tree = Tree()
    private val rootNode = DefaultMutableTreeNode("Dependencies")
    private val treeModel = DefaultTreeModel(rootNode)
    private val statusLabel = JBLabel("Loading...")

    init {
        tree.model = treeModel
        tree.isRootVisible = true
        tree.cellRenderer = DependencyTreeCellRenderer()

        // Create toolbar
        val toolbar = createToolbar()
        add(toolbar.component, BorderLayout.NORTH)

        // Add tree in scroll pane
        add(JBScrollPane(tree), BorderLayout.CENTER)

        // Status bar
        statusLabel.border = JBUI.Borders.empty(4)
        add(statusLabel, BorderLayout.SOUTH)

        // Load dependencies
        loadDependencies()
    }

    private fun createToolbar(): ActionToolbar {
        val actionGroup = DefaultActionGroup().apply {
            add(RefreshAction())
            add(ExpandAllAction())
            add(CollapseAllAction())
        }

        return ActionManager.getInstance()
            .createActionToolbar("CcgoDependencyTree", actionGroup, true)
            .apply { targetComponent = this@DependencyTreePanel }
    }

    fun loadDependencies() {
        statusLabel.text = "Loading dependencies..."
        statusLabel.foreground = JBColor.GRAY

        ApplicationManager.getApplication().executeOnPooledThread {
            try {
                val dependencies = fetchDependencies()
                ApplicationManager.getApplication().invokeLater {
                    updateTree(dependencies)
                    statusLabel.text = "Dependencies loaded"
                    statusLabel.foreground = JBColor.GRAY
                }
            } catch (e: Exception) {
                ApplicationManager.getApplication().invokeLater {
                    statusLabel.text = "Error: ${e.message}"
                    statusLabel.foreground = JBColor.RED
                    rootNode.removeAllChildren()
                    rootNode.add(DefaultMutableTreeNode("Failed to load dependencies: ${e.message}"))
                    treeModel.reload()
                }
            }
        }
    }

    private fun fetchDependencies(): JsonObject {
        val ccgoPath = CcgoSettings.getInstance().executablePath
        val projectRoot = CcgoProjectService.getInstance(project).getProjectRoot()
            ?: throw Exception("Project root not found")

        val processBuilder = ProcessBuilder(ccgoPath, "tree", "--format", "json")
            .directory(java.io.File(projectRoot))
            .redirectErrorStream(true)

        val process = processBuilder.start()
        val reader = BufferedReader(InputStreamReader(process.inputStream))
        val output = reader.readText()
        reader.close()

        val exitCode = process.waitFor()
        if (exitCode != 0) {
            throw Exception("ccgo tree failed with exit code $exitCode")
        }

        return Gson().fromJson(output, JsonObject::class.java)
    }

    private fun updateTree(projectTree: JsonObject) {
        rootNode.removeAllChildren()

        // The root is the project itself
        val projectName = projectTree.get("name")?.asString ?: "Project"
        val projectVersion = projectTree.get("version")?.asString ?: ""
        val displayName = if (projectVersion.isNotEmpty()) "$projectName@$projectVersion" else projectName
        rootNode.userObject = displayName

        // Add dependencies
        val dependencies = projectTree.get("dependencies")?.asJsonArray
        dependencies?.forEach { dep ->
            val node = createDependencyNode(dep.asJsonObject)
            rootNode.add(node)
        }

        treeModel.reload()
        expandAll()
    }

    private fun createDependencyNode(dep: JsonObject): DefaultMutableTreeNode {
        val name = dep.get("name")?.asString ?: "unknown"
        val version = dep.get("version")?.asString ?: ""
        val source = dep.get("source")?.asString ?: ""

        val displayName = if (version.isNotEmpty()) "$name@$version" else name
        val node = DefaultMutableTreeNode(DependencyInfo(displayName, source))

        // Add transitive dependencies
        val transitive = dep.get("dependencies")?.asJsonArray
        transitive?.forEach { child ->
            node.add(createDependencyNode(child.asJsonObject))
        }

        return node
    }

    private fun expandAll() {
        var row = 0
        while (row < tree.rowCount) {
            tree.expandRow(row)
            row++
        }
    }

    private fun collapseAll() {
        var row = tree.rowCount - 1
        while (row >= 0) {
            tree.collapseRow(row)
            row--
        }
    }

    /**
     * Data class for dependency information.
     */
    data class DependencyInfo(val name: String, val source: String) {
        override fun toString(): String = name
    }

    // Actions
    private inner class RefreshAction : AnAction("Refresh", "Refresh dependencies", AllIcons.Actions.Refresh) {
        override fun actionPerformed(e: AnActionEvent) {
            loadDependencies()
        }
    }

    private inner class ExpandAllAction : AnAction("Expand All", "Expand all nodes", AllIcons.Actions.Expandall) {
        override fun actionPerformed(e: AnActionEvent) {
            expandAll()
        }
    }

    private inner class CollapseAllAction : AnAction("Collapse All", "Collapse all nodes", AllIcons.Actions.Collapseall) {
        override fun actionPerformed(e: AnActionEvent) {
            collapseAll()
        }
    }
}
