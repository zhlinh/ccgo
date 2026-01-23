package com.ccgo.plugin

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity

/**
 * Startup activity that runs when a project is opened.
 * Detects CCGO projects and shows relevant notifications.
 */
class CcgoStartupActivity : ProjectActivity {

    override suspend fun execute(project: Project) {
        val service = CcgoProjectService.getInstance(project)

        if (service.hasCcgoToml()) {
            // Project has CCGO.toml - this is a CCGO project
            showWelcomeNotification(project)
        }
    }

    private fun showWelcomeNotification(project: Project) {
        val settings = com.ccgo.plugin.settings.CcgoSettings.getInstance()
        if (!settings.showNotifications) return

        NotificationGroupManager.getInstance()
            .getNotificationGroup("CCGO Notifications")
            .createNotification(
                "CCGO Project Detected",
                "This project uses CCGO. Use Tools > CCGO to build, test, and manage dependencies.",
                NotificationType.INFORMATION
            )
            .notify(project)
    }
}
