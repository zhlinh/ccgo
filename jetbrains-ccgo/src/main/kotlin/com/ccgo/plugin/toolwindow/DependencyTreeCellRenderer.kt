package com.ccgo.plugin.toolwindow

import com.ccgo.plugin.CcgoIcons
import com.intellij.icons.AllIcons
import com.intellij.ui.ColoredTreeCellRenderer
import com.intellij.ui.SimpleTextAttributes
import javax.swing.JTree
import javax.swing.tree.DefaultMutableTreeNode

/**
 * Custom cell renderer for the dependency tree.
 */
class DependencyTreeCellRenderer : ColoredTreeCellRenderer() {

    override fun customizeCellRenderer(
        tree: JTree,
        value: Any?,
        selected: Boolean,
        expanded: Boolean,
        leaf: Boolean,
        row: Int,
        hasFocus: Boolean
    ) {
        val node = value as? DefaultMutableTreeNode ?: return
        val userObject = node.userObject

        when (userObject) {
            is String -> {
                // Root node or simple string
                icon = CcgoIcons.CCGO_13
                append(userObject, SimpleTextAttributes.REGULAR_BOLD_ATTRIBUTES)
            }
            is DependencyTreePanel.DependencyInfo -> {
                // Dependency node
                icon = if (leaf) CcgoIcons.DEPENDENCY else AllIcons.Nodes.Package

                // Parse name and version
                val parts = userObject.name.split("@")
                val name = parts[0]
                val version = parts.getOrNull(1)

                append(name, SimpleTextAttributes.REGULAR_ATTRIBUTES)

                if (version != null) {
                    append("@", SimpleTextAttributes.GRAYED_ATTRIBUTES)
                    append(version, SimpleTextAttributes.GRAYED_BOLD_ATTRIBUTES)
                }

                // Show source as tooltip
                if (userObject.source.isNotEmpty()) {
                    toolTipText = "Source: ${userObject.source}"
                }
            }
            else -> {
                append(userObject.toString(), SimpleTextAttributes.REGULAR_ATTRIBUTES)
            }
        }
    }
}
