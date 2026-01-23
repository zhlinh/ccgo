package com.ccgo.plugin.run

import com.ccgo.plugin.CcgoProjectService
import com.ccgo.plugin.settings.CcgoSettings
import com.intellij.execution.ExecutionException
import com.intellij.execution.Executor
import com.intellij.execution.configurations.*
import com.intellij.execution.process.OSProcessHandler
import com.intellij.execution.process.ProcessHandler
import com.intellij.execution.process.ProcessHandlerFactory
import com.intellij.execution.process.ProcessTerminatedListener
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import java.io.File

/**
 * Run configuration for CCGO commands.
 */
class CcgoRunConfiguration(
    project: Project,
    factory: ConfigurationFactory,
    name: String
) : RunConfigurationBase<CcgoRunConfigurationOptions>(project, factory, name) {

    override fun getOptions(): CcgoRunConfigurationOptions {
        return super.getOptions() as CcgoRunConfigurationOptions
    }

    var command: String
        get() = options.command
        set(value) { options.command = value }

    var platform: String
        get() = options.platform
        set(value) { options.platform = value }

    var release: Boolean
        get() = options.release
        set(value) { options.release = value }

    var architectures: String
        get() = options.architectures
        set(value) { options.architectures = value }

    var ideProject: Boolean
        get() = options.ideProject
        set(value) { options.ideProject = value }

    var additionalArgs: String
        get() = options.additionalArgs
        set(value) { options.additionalArgs = value }

    override fun getConfigurationEditor(): SettingsEditor<out RunConfiguration> {
        return CcgoRunConfigurationEditor(project)
    }

    override fun checkConfiguration() {
        val service = CcgoProjectService.getInstance(project)
        if (!service.hasCcgoToml()) {
            throw RuntimeConfigurationError("No CCGO.toml found in project")
        }
    }

    override fun getState(executor: Executor, environment: ExecutionEnvironment): RunProfileState {
        return CcgoCommandLineState(this, environment)
    }

    /**
     * Build the command line arguments for the CCGO command.
     */
    fun buildCommandLineArgs(): List<String> {
        val args = mutableListOf(command)

        // Add platform for build/test commands
        if (command in listOf("build", "test", "bench")) {
            args.add(platform)
        }

        if (release) {
            args.add("--release")
        }

        if (ideProject && command == "build") {
            args.add("--ide-project")
        }

        if (architectures.isNotBlank()) {
            args.add("--arch")
            args.add(architectures)
        }

        if (additionalArgs.isNotBlank()) {
            args.addAll(additionalArgs.split("\\s+".toRegex()))
        }

        return args
    }
}

/**
 * Command line state for executing CCGO commands.
 */
class CcgoCommandLineState(
    private val configuration: CcgoRunConfiguration,
    environment: ExecutionEnvironment
) : CommandLineState(environment) {

    override fun startProcess(): ProcessHandler {
        val settings = CcgoSettings.getInstance()
        val projectRoot = CcgoProjectService.getInstance(configuration.project).getProjectRoot()
            ?: throw ExecutionException("Project root not found")

        val commandLine = GeneralCommandLine()
            .withExePath(settings.executablePath)
            .withParameters(configuration.buildCommandLineArgs())
            .withWorkDirectory(File(projectRoot))
            .withEnvironment(System.getenv())

        val handler = ProcessHandlerFactory.getInstance()
            .createColoredProcessHandler(commandLine)
        ProcessTerminatedListener.attach(handler)

        return handler
    }
}
