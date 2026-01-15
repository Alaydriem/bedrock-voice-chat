plugins {
    java
    id("com.gradleup.shadow")
}

// Apply custom RunHytale plugin for testing
apply<RunHytalePlugin>()

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-hytale")
}

// Conditional compilation - only build if HytaleServer.jar exists
val hytaleJar = file("libs/HytaleServer.jar")

if (hytaleJar.exists()) {
    dependencies {
        // Common module - will be shadowed
        implementation(project(":common"))

        // Hytale Server (local dependency, provided at runtime)
        compileOnly(files("libs/HytaleServer.jar"))

        // Test dependencies
        testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
        testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    }

    tasks.test {
        useJUnitPlatform()
    }

    tasks.shadowJar {
        archiveClassifier.set("")

        // Relocate Gson to avoid conflicts
        relocate("com.google.gson", "com.alaydriem.bedrockvoicechat.shaded.gson")

        // Include common module and its dependencies (Gson)
        dependencies {
            include(project(":common"))
            include(dependency("com.google.code.gson:gson"))
        }

        from("LICENSE") {
            rename { "${it}_${archivesBaseName}" }
        }
    }

    // Disable default jar task - use shadowJar instead
    tasks.jar {
        enabled = false
    }

    // Make build depend on shadowJar
    tasks.build {
        dependsOn(tasks.shadowJar)
    }
} else {
    logger.warn("HytaleServer.jar not found at ${hytaleJar.absolutePath} - Hytale module will not be compiled")

    tasks.withType<JavaCompile>().configureEach {
        enabled = false
    }

    tasks.withType<Jar>().configureEach {
        enabled = false
    }
}
