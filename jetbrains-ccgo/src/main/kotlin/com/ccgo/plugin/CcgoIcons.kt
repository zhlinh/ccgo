package com.ccgo.plugin

import com.intellij.openapi.util.IconLoader
import javax.swing.Icon

/**
 * Icon definitions for the CCGO plugin.
 */
object CcgoIcons {
    @JvmField
    val CCGO: Icon = IconLoader.getIcon("/icons/ccgo.svg", CcgoIcons::class.java)

    @JvmField
    val CCGO_13: Icon = IconLoader.getIcon("/icons/ccgo_13.svg", CcgoIcons::class.java)

    @JvmField
    val DEPENDENCY: Icon = IconLoader.getIcon("/icons/dependency.svg", CcgoIcons::class.java)

    @JvmField
    val PLATFORM: Icon = IconLoader.getIcon("/icons/platform.svg", CcgoIcons::class.java)
}
