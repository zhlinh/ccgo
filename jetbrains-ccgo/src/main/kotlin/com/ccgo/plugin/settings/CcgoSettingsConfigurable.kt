package com.ccgo.plugin.settings

import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.BoundConfigurable
import com.intellij.openapi.ui.DialogPanel
import com.intellij.ui.dsl.builder.*

/**
 * Settings UI for CCGO plugin configuration.
 * Accessible via Settings/Preferences > Tools > CCGO
 */
class CcgoSettingsConfigurable : BoundConfigurable("CCGO") {

    private val settings = CcgoSettings.getInstance()

    override fun createPanel(): DialogPanel = panel {
        group("General") {
            row("Executable path:") {
                textFieldWithBrowseButton(
                    browseDialogTitle = "Select CCGO Executable",
                    fileChooserDescriptor = FileChooserDescriptorFactory.createSingleFileDescriptor()
                )
                    .bindText(settings::executablePath)
                    .columns(COLUMNS_LARGE)
                    .comment("Path to the CCGO executable. Leave as 'ccgo' if it's in your PATH.")
            }

            row("Default platform:") {
                comboBox(CcgoSettings.PLATFORMS)
                    .bindItem(
                        getter = { CcgoSettings.PLATFORMS.find { it.first == settings.defaultPlatform } ?: CcgoSettings.PLATFORMS[0] },
                        setter = { settings.defaultPlatform = it?.first ?: "" }
                    )
                    .comment("Default target platform for builds. Auto-detect will use the current OS.")
            }
        }

        group("Behavior") {
            row {
                checkBox("Auto-refresh dependencies")
                    .bindSelected(settings::autoRefreshDependencies)
                    .comment("Automatically refresh the dependency tree when CCGO.toml changes")
            }

            row {
                checkBox("Show notifications")
                    .bindSelected(settings::showNotifications)
                    .comment("Show balloon notifications for build completion and errors")
            }

            row {
                checkBox("Run builds in terminal")
                    .bindSelected(settings::buildInTerminal)
                    .comment("Execute builds in the integrated terminal instead of run tool window")
            }
        }

        group("About") {
            row {
                browserLink("CCGO Documentation", "https://github.com/zhlinh/ccgo")
            }
            row {
                browserLink("Report an Issue", "https://github.com/zhlinh/ccgo/issues")
            }
        }
    }

    override fun getDisplayName(): String = "CCGO"
}
