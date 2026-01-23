plugins {
    java
    kotlin("jvm") version "2.0.21" apply false
    id("com.gradleup.shadow") version "9.0.0" apply false
}

// =============================================================================
// Dev Build Task - Full rebuild workflow for local development
// =============================================================================
// Usage:
//   ./gradlew devBuild
//   ./gradlew devBuild -PfabricDest=/path/to/mods -PpaperDest=/path/to/plugins -PhytaleDest=/path/to/plugins
//   ./gradlew devBuild -Prelease  (for release builds, default is debug)
//
// This task:
//   1. Builds the Rust native library (cargo build --lib)
//   2. Copies native library to resources (copyNativeWindows)
//   3. Builds all 3 mod JARs (fabric, paper, hytale)
//   4. Copies JARs to specified destinations (if provided)
// =============================================================================

val bvcRoot = projectDir.parentFile.parentFile
val rustServerDir = File(bvcRoot, "server/server")

// Task to build Rust native library
tasks.register<Exec>("buildRustLibrary") {
    group = "build"
    description = "Build the Rust BVC server library"

    workingDir = rustServerDir

    val isRelease = project.hasProperty("release")
    val args = mutableListOf("cargo", "build", "--lib")
    if (isRelease) {
        args.add("--release")
    }
    commandLine(args)

    doFirst {
        val mode = if (isRelease) "release" else "debug"
        logger.lifecycle("Building Rust library ($mode) in ${rustServerDir.absolutePath}")
    }
}

// Main dev build task
tasks.register("devBuild") {
    group = "build"
    description = "Full dev build: Rust library -> copy native -> build all mods -> copy to destinations"

    // Step 1: Build Rust library
    dependsOn("buildRustLibrary")

    // Step 2: Copy native library (runs after Rust build)
    dependsOn(":common:copyNativeWindows")

    // Step 3: Build all mods (run after native copy)
    dependsOn(":paper:shadowJar", ":hytale:shadowJar")
}

// Configure task ordering after all projects are evaluated
gradle.projectsEvaluated {
    tasks.findByPath(":common:copyNativeWindows")?.mustRunAfter(":buildRustLibrary")
    tasks.findByPath(":paper:shadowJar")?.mustRunAfter(":common:copyNativeWindows")
    tasks.findByPath(":hytale:shadowJar")?.mustRunAfter(":common:copyNativeWindows")
}

tasks.named("devBuild") {

    doLast {
        val modVersion: String by project
        val archivesBaseName: String by project

        // Collect built JARs
        val fabricJar = file("fabric/build/libs/${archivesBaseName}-${modVersion}.jar")
        val paperJar = file("paper/build/libs/${archivesBaseName}-paper-${modVersion}.jar")
        val hytaleJar = file("hytale/build/libs/${archivesBaseName}-hytale-${modVersion}.jar")

        logger.lifecycle("")
        logger.lifecycle("=== Build Complete ===")
        logger.lifecycle("Fabric: ${if (fabricJar.exists()) fabricJar.absolutePath else "NOT BUILT (run separately: cd fabric && ./gradlew build)"}")
        logger.lifecycle("Paper:  ${if (paperJar.exists()) paperJar.absolutePath else "NOT FOUND"}")
        logger.lifecycle("Hytale: ${if (hytaleJar.exists()) hytaleJar.absolutePath else "NOT FOUND"}")

        // Copy to destinations if specified
        val fabricDest = project.findProperty("fabricDest")?.toString()
        val paperDest = project.findProperty("paperDest")?.toString()
        val hytaleDest = project.findProperty("hytaleDest")?.toString()

        if (fabricDest != null || paperDest != null || hytaleDest != null) {
            logger.lifecycle("")
            logger.lifecycle("=== Copying to Destinations ===")
        }

        if (fabricDest != null && fabricJar.exists()) {
            val destDir = File(fabricDest)
            destDir.mkdirs()
            val destFile = File(destDir, fabricJar.name)
            fabricJar.copyTo(destFile, overwrite = true)
            logger.lifecycle("Fabric -> $destFile")
        }

        if (paperDest != null && paperJar.exists()) {
            val destDir = File(paperDest)
            destDir.mkdirs()
            val destFile = File(destDir, paperJar.name)
            paperJar.copyTo(destFile, overwrite = true)
            logger.lifecycle("Paper  -> $destFile")
        }

        if (hytaleDest != null && hytaleJar.exists()) {
            val destDir = File(hytaleDest)
            destDir.mkdirs()
            val destFile = File(destDir, hytaleJar.name)
            hytaleJar.copyTo(destFile, overwrite = true)
            logger.lifecycle("Hytale -> $destFile")
        }
    }
}

// Convenience task that also builds fabric (requires separate Gradle invocation due to composite build)
tasks.register<Exec>("devBuildAll") {
    group = "build"
    description = "Full dev build including Fabric (which requires separate Gradle invocation)"

    dependsOn("devBuild")
    mustRunAfter("devBuild")

    workingDir = file("fabric")
    commandLine("cmd", "/c", "..\\gradlew.bat", "build")

    doFirst {
        logger.lifecycle("")
        logger.lifecycle("=== Building Fabric (separate Gradle project) ===")
    }

    doLast {
        val modVersion: String by project
        val archivesBaseName: String by project
        val fabricJar = file("fabric/build/libs/${archivesBaseName}-${modVersion}.jar")
        val fabricDest = project.findProperty("fabricDest")?.toString()

        if (fabricDest != null && fabricJar.exists()) {
            val destDir = File(fabricDest)
            destDir.mkdirs()
            val destFile = File(destDir, fabricJar.name)
            fabricJar.copyTo(destFile, overwrite = true)
            logger.lifecycle("Fabric -> $destFile")
        }
    }
}

val modVersion: String by project
val mavenGroup: String by project

allprojects {
    group = mavenGroup
    version = modVersion
}

subprojects {
    apply(plugin = "java")
    apply(plugin = "org.jetbrains.kotlin.jvm")

    repositories {
        mavenCentral()
        maven {
            name = "Fabric"
            url = uri("https://maven.fabricmc.net/")
        }
        maven {
            name = "PaperMC"
            url = uri("https://repo.papermc.io/repository/maven-public/")
        }
    }

    java {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    tasks.withType<JavaCompile>().configureEach {
        options.release.set(21)
        options.encoding = "UTF-8"
    }

    tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
        compilerOptions {
            jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
        }
    }

    dependencies {
        "implementation"(kotlin("stdlib"))
    }
}
