package com.ccgo.plugin.run

import com.ccgo.plugin.CcgoIcons
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationType
import com.intellij.execution.configurations.ConfigurationTypeUtil
import javax.swing.Icon

/**
 * Run configuration type for CCGO builds.
 */
class CcgoRunConfigurationType : ConfigurationType {

    override fun getDisplayName(): String = "CCGO Build"

    override fun getConfigurationTypeDescription(): String = "Run CCGO build commands"

    override fun getIcon(): Icon = CcgoIcons.CCGO

    override fun getId(): String = "CCGO_RUN_CONFIGURATION"

    override fun getConfigurationFactories(): Array<ConfigurationFactory> {
        return arrayOf(CcgoRunConfigurationFactory(this))
    }

    companion object {
        @JvmStatic
        fun getInstance(): CcgoRunConfigurationType {
            return ConfigurationTypeUtil.findConfigurationType(CcgoRunConfigurationType::class.java)
        }
    }
}
