package com.ccgo.plugin.actions

import com.ccgo.plugin.CcgoProjectService
import com.ccgo.plugin.settings.CcgoSettings
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.executors.DefaultRunExecutor
import com.intellij.execution.filters.TextConsoleBuilderFactory
import com.intellij.execution.process.OSProcessHandler
import com.intellij.execution.process.ProcessAdapter
import com.intellij.execution.process.ProcessEvent
import com.intellij.execution.ui.ConsoleView
import com.intellij.execution.ui.RunContentDescriptor
import com.intellij.execution.ui.RunContentManager
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindowManager
import java.io.File

/**
 * Utility class for executing CCGO commands.
 */
object CcgoCommandExecutor {

    /**
     * Execute a CCGO command in the terminal or run tool window.
     */
    fun execute(project: Project, args: List<String>) {
        val settings = CcgoSettings.getInstance()
        val projectRoot = CcgoProjectService.getInstance(project).getProjectRoot() ?: return

        val commandLine = GeneralCommandLine()
            .withExePath(settings.executablePath)
            .withParameters(args)
            .withWorkDirectory(File(projectRoot))
            .withEnvironment(System.getenv())

        if (settings.buildInTerminal) {
            executeInTerminal(project, commandLine)
        } else {
            executeInRunWindow(project, commandLine, args)
        }
    }

    private fun executeInTerminal(project: Project, commandLine: GeneralCommandLine) {
        ApplicationManager.getApplication().invokeLater {
            try {
                val command = "${commandLine.exePath} ${commandLine.parametersList.parametersString}"

                // Try to use the terminal tool window
                val toolWindowManager = ToolWindowManager.getInstance(project)
                val terminalWindow = toolWindowManager.getToolWindow("Terminal")

                if (terminalWindow != null) {
                    terminalWindow.activate {
                        // Show notification with the command to execute
                        showNotification(
                            project,
                            "CCGO Command",
                            "Execute in terminal: $command",
                            NotificationType.INFORMATION
                        )
                    }
                } else {
                    // Fallback to run window
                    executeInRunWindow(project, commandLine, commandLine.parametersList.parameters)
                }
            } catch (e: Exception) {
                showNotification(
                    project,
                    "CCGO Error",
                    "Failed to execute command: ${e.message}",
                    NotificationType.ERROR
                )
            }
        }
    }

    private fun executeInRunWindow(project: Project, commandLine: GeneralCommandLine, args: List<String>) {
        ApplicationManager.getApplication().invokeLater {
            try {
                // Create console view
                val consoleBuilder = TextConsoleBuilderFactory.getInstance().createBuilder(project)
                val console: ConsoleView = consoleBuilder.console

                // Create process handler
                val handler = OSProcessHandler(commandLine)
                console.attachToProcess(handler)

                // Create run content descriptor
                val runContent = RunContentDescriptor(
                    console,
                    handler,
                    console.component,
                    "CCGO: ${args.joinToString(" ")}"
                )

                // Show in Run tool window
                RunContentManager.getInstance(project).showRunContent(
                    DefaultRunExecutor.getRunExecutorInstance(),
                    runContent
                )

                handler.addProcessListener(object : ProcessAdapter() {
                    override fun processTerminated(event: ProcessEvent) {
                        val exitCode = event.exitCode
                        if (CcgoSettings.getInstance().showNotifications) {
                            if (exitCode == 0) {
                                showNotification(
                                    project,
                                    "CCGO",
                                    "Command completed successfully",
                                    NotificationType.INFORMATION
                                )
                            } else {
                                showNotification(
                                    project,
                                    "CCGO",
                                    "Command failed with exit code $exitCode",
                                    NotificationType.ERROR
                                )
                            }
                        }
                    }
                })

                handler.startNotify()
            } catch (e: Exception) {
                showNotification(
                    project,
                    "CCGO Error",
                    "Failed to execute command: ${e.message}",
                    NotificationType.ERROR
                )
            }
        }
    }

    private fun showNotification(project: Project, title: String, content: String, type: NotificationType) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup("CCGO Notifications")
            .createNotification(title, content, type)
            .notify(project)
    }
}
