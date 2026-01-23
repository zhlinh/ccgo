package com.ccgo.plugin.completion

import com.intellij.codeInsight.template.TemplateActionContext
import com.intellij.codeInsight.template.TemplateContextType

/**
 * Template context for CCGO.toml files.
 * Enables live templates specifically for CCGO configuration files.
 */
class CcgoTomlContext : TemplateContextType("CCGO_TOML", "CCGO Configuration") {

    override fun isInContext(templateActionContext: TemplateActionContext): Boolean {
        val file = templateActionContext.file
        return file.name == "CCGO.toml"
    }
}
