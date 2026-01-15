plugins {
    java
}

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-hytale")
}

// Conditional compilation - only build if HytaleServer.jar exists
val hytaleJar = file("libs/HytaleServer.jar")

if (hytaleJar.exists()) {
    dependencies {
        // Common module
        implementation(project(":common"))

        // Hytale Server (local dependency)
        compileOnly(files("libs/HytaleServer.jar"))
    }

    tasks.jar {
        // Include common module in the jar
        from(project(":common").sourceSets.main.get().output)

        from("LICENSE") {
            rename { "${it}_${archivesBaseName}" }
        }
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
