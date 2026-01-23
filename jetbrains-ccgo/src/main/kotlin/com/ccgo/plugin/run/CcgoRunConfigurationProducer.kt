package com.ccgo.plugin.run

import com.ccgo.plugin.CcgoProjectService
import com.intellij.execution.actions.ConfigurationContext
import com.intellij.execution.actions.LazyRunConfigurationProducer
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.openapi.util.Ref
import com.intellij.psi.PsiElement

/**
 * Producer for creating CCGO run configurations from context.
 */
class CcgoRunConfigurationProducer : LazyRunConfigurationProducer<CcgoRunConfiguration>() {

    override fun getConfigurationFactory(): ConfigurationFactory {
        return CcgoRunConfigurationType.getInstance().configurationFactories[0]
    }

    override fun setupConfigurationFromContext(
        configuration: CcgoRunConfiguration,
        context: ConfigurationContext,
        sourceElement: Ref<PsiElement>
    ): Boolean {
        val project = context.project
        val service = CcgoProjectService.getInstance(project)

        if (!service.hasCcgoToml()) {
            return false
        }

        // Set default platform based on current OS
        configuration.platform = service.detectCurrentPlatform()
        configuration.name = "CCGO Build (${configuration.platform})"

        return true
    }

    override fun isConfigurationFromContext(
        configuration: CcgoRunConfiguration,
        context: ConfigurationContext
    ): Boolean {
        val project = context.project
        val service = CcgoProjectService.getInstance(project)

        return service.hasCcgoToml() &&
                configuration.command == "build" &&
                configuration.platform == service.detectCurrentPlatform()
    }
}
