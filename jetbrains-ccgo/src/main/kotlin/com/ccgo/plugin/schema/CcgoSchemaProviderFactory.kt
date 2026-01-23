package com.ccgo.plugin.schema

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.jetbrains.jsonSchema.extension.JsonSchemaFileProvider
import com.jetbrains.jsonSchema.extension.JsonSchemaProviderFactory
import com.jetbrains.jsonSchema.extension.SchemaType

/**
 * Factory for providing JSON Schema validation for CCGO.toml files.
 */
class CcgoSchemaProviderFactory : JsonSchemaProviderFactory {

    override fun getProviders(project: Project): List<JsonSchemaFileProvider> {
        return listOf(CcgoSchemaProvider())
    }
}

/**
 * JSON Schema provider for CCGO.toml files.
 * Provides validation and auto-completion based on the CCGO schema.
 */
class CcgoSchemaProvider : JsonSchemaFileProvider {

    override fun isAvailable(file: VirtualFile): Boolean {
        return file.name == "CCGO.toml"
    }

    override fun getName(): String = "CCGO Configuration Schema"

    override fun getSchemaFile(): VirtualFile? {
        val resource = this::class.java.getResource("/schemas/ccgo.schema.json")
        return resource?.let {
            com.intellij.openapi.vfs.VfsUtil.findFileByURL(it)
        }
    }

    override fun getSchemaType(): SchemaType = SchemaType.embeddedSchema

    override fun getRemoteSource(): String? = null

    override fun getPresentableName(): String = "CCGO"
}
