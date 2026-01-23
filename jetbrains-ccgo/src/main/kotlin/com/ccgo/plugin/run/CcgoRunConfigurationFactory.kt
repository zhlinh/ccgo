package com.ccgo.plugin.run

import com.ccgo.plugin.CcgoIcons
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationType
import com.intellij.execution.configurations.RunConfiguration
import com.intellij.openapi.components.BaseState
import com.intellij.openapi.project.Project
import javax.swing.Icon

/**
 * Factory for creating CCGO run configurations.
 */
class CcgoRunConfigurationFactory(type: ConfigurationType) : ConfigurationFactory(type) {

    override fun getId(): String = "CCGO_RUN_CONFIGURATION_FACTORY"

    override fun createTemplateConfiguration(project: Project): RunConfiguration {
        return CcgoRunConfiguration(project, this, "CCGO Build")
    }

    override fun getOptionsClass(): Class<out BaseState> = CcgoRunConfigurationOptions::class.java

    override fun getIcon(): Icon = CcgoIcons.CCGO
}
