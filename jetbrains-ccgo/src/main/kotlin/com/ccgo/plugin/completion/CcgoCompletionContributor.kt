package com.ccgo.plugin.completion

import com.intellij.codeInsight.completion.*
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.patterns.PlatformPatterns
import com.intellij.psi.PsiElement
import com.intellij.util.ProcessingContext

/**
 * Completion contributor for CCGO.toml files.
 * Provides auto-completion for CCGO-specific sections and values.
 */
class CcgoCompletionContributor : CompletionContributor() {

    init {
        // Add completions for TOML files
        extend(
            CompletionType.BASIC,
            PlatformPatterns.psiElement(),
            CcgoCompletionProvider()
        )
    }
}

/**
 * Provides completion suggestions for CCGO.toml.
 */
class CcgoCompletionProvider : CompletionProvider<CompletionParameters>() {

    override fun addCompletions(
        parameters: CompletionParameters,
        context: ProcessingContext,
        result: CompletionResultSet
    ) {
        val file = parameters.originalFile
        if (file.name != "CCGO.toml") return

        val position = parameters.position
        val text = position.text

        // Determine context and provide appropriate completions
        when {
            isInRootContext(position) -> addSectionCompletions(result)
            isInPlatformContext(position) -> addPlatformCompletions(result)
            isInBuildContext(position) -> addBuildCompletions(result)
        }
    }

    private fun isInRootContext(element: PsiElement): Boolean {
        // Check if we're at the root level of the TOML file
        return element.parent?.parent?.parent == null ||
                element.text.startsWith("[")
    }

    private fun isInPlatformContext(element: PsiElement): Boolean {
        // Check if we're in a platform-related context
        val text = element.containingFile?.text ?: return false
        val offset = element.textOffset
        val beforeText = text.substring(0, offset)
        return beforeText.contains("[platforms.") || beforeText.contains("platforms =")
    }

    private fun isInBuildContext(element: PsiElement): Boolean {
        val text = element.containingFile?.text ?: return false
        val offset = element.textOffset
        val beforeText = text.substring(0, offset)
        return beforeText.contains("[build]")
    }

    private fun addSectionCompletions(result: CompletionResultSet) {
        val sections = listOf(
            "[package]" to "Package metadata section",
            "[build]" to "Build configuration section",
            "[dependencies]" to "Project dependencies",
            "[dev-dependencies]" to "Development dependencies",
            "[platforms]" to "Platform-specific configurations",
            "[platforms.android]" to "Android platform settings",
            "[platforms.ios]" to "iOS platform settings",
            "[platforms.macos]" to "macOS platform settings",
            "[platforms.linux]" to "Linux platform settings",
            "[platforms.windows]" to "Windows platform settings",
            "[platforms.ohos]" to "OpenHarmony platform settings",
            "[publish]" to "Publishing configuration",
            "[publish.maven]" to "Maven publishing settings",
            "[publish.cocoapods]" to "CocoaPods publishing settings",
            "[publish.spm]" to "Swift Package Manager settings"
        )

        sections.forEach { (section, description) ->
            result.addElement(
                LookupElementBuilder.create(section)
                    .withTypeText(description)
                    .withInsertHandler { context, _ ->
                        context.document.insertString(context.tailOffset, "\n")
                        context.editor.caretModel.moveToOffset(context.tailOffset)
                    }
            )
        }
    }

    private fun addPlatformCompletions(result: CompletionResultSet) {
        val platforms = listOf(
            "android" to "Android platform",
            "ios" to "iOS platform",
            "macos" to "macOS platform",
            "linux" to "Linux platform",
            "windows" to "Windows platform",
            "ohos" to "OpenHarmony platform",
            "kmp" to "Kotlin Multiplatform"
        )

        platforms.forEach { (platform, description) ->
            result.addElement(
                LookupElementBuilder.create(platform)
                    .withTypeText(description)
            )
        }
    }

    private fun addBuildCompletions(result: CompletionResultSet) {
        val buildOptions = listOf(
            "cmake_minimum_version" to "Minimum CMake version",
            "cpp_standard" to "C++ standard (11, 14, 17, 20)",
            "c_standard" to "C standard (99, 11, 17)",
            "shared" to "Build shared library",
            "static" to "Build static library",
            "pic" to "Position-independent code",
            "defines" to "Preprocessor definitions"
        )

        buildOptions.forEach { (option, description) ->
            result.addElement(
                LookupElementBuilder.create(option)
                    .withTypeText(description)
                    .withInsertHandler { context, _ ->
                        context.document.insertString(context.tailOffset, " = ")
                        context.editor.caretModel.moveToOffset(context.tailOffset)
                    }
            )
        }
    }
}
