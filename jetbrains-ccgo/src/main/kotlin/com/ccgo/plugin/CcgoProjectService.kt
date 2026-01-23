package com.ccgo.plugin

import com.intellij.openapi.components.Service
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.VirtualFileManager
import java.io.File

/**
 * Project-level service for CCGO plugin.
 * Provides project-specific functionality like detecting CCGO.toml files.
 */
@Service(Service.Level.PROJECT)
class CcgoProjectService(private val project: Project) {

    /**
     * Check if the project contains a CCGO.toml file.
     */
    fun hasCcgoToml(): Boolean {
        val basePath = project.basePath ?: return false
        return File(basePath, "CCGO.toml").exists()
    }

    /**
     * Get the CCGO.toml virtual file if it exists.
     */
    fun getCcgoTomlFile(): VirtualFile? {
        val basePath = project.basePath ?: return null
        val file = File(basePath, "CCGO.toml")
        if (!file.exists()) return null
        return VirtualFileManager.getInstance().findFileByUrl("file://${file.absolutePath}")
    }

    /**
     * Get the project root directory.
     */
    fun getProjectRoot(): String? = project.basePath

    /**
     * Detect the current platform based on the OS.
     */
    fun detectCurrentPlatform(): String {
        val os = System.getProperty("os.name").lowercase()
        return when {
            os.contains("mac") -> "macos"
            os.contains("win") -> "windows"
            os.contains("linux") -> "linux"
            else -> "linux"
        }
    }

    companion object {
        @JvmStatic
        fun getInstance(project: Project): CcgoProjectService {
            return project.getService(CcgoProjectService::class.java)
        }
    }
}
