plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "1.9.22"
    id("org.jetbrains.intellij") version "1.17.0"
}

group = "com.ccgo"
version = "0.1.0"

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.google.code.gson:gson:2.10.1")
}

kotlin {
    jvmToolchain(17)
}

intellij {
    version.set("2024.1")
    type.set("IC") // IntelliJ IDEA Community Edition
    plugins.set(listOf(
        "org.toml.lang:241.14494.150", // TOML language support
        "com.intellij.java"
    ))
}

tasks {
    patchPluginXml {
        sinceBuild.set("241")
        untilBuild.set("251.*")
        changeNotes.set("""
            <h2>0.1.0</h2>
            <ul>
                <li>Initial release</li>
                <li>CCGO.toml syntax highlighting and validation</li>
                <li>Dependency tree visualization</li>
                <li>Build and test run configurations</li>
                <li>Code snippets for common patterns</li>
            </ul>
        """.trimIndent())
    }

    buildSearchableOptions {
        enabled = false
    }

    runIde {
        // Uncomment to use CLion instead of IntelliJ IDEA
        // ideDir.set(file("/Applications/CLion.app/Contents"))
    }

    signPlugin {
        certificateChain.set(System.getenv("CERTIFICATE_CHAIN"))
        privateKey.set(System.getenv("PRIVATE_KEY"))
        password.set(System.getenv("PRIVATE_KEY_PASSWORD"))
    }

    publishPlugin {
        token.set(System.getenv("PUBLISH_TOKEN"))
    }
}
