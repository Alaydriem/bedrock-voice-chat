import org.gradle.api.DefaultTask
import org.gradle.api.tasks.TaskAction
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.Exec
import org.gradle.process.ExecOperations
import javax.inject.Inject

plugins {
    java
    id("com.gradleup.shadow")
}

val archivesBaseName: String by project
val hytaleJar = file("libs/HytaleServer.jar")

base {
    archivesName.set("$archivesBaseName-hytale")
}

// Task to download Hytale server if not present
abstract class DownloadHytaleServerTask : DefaultTask() {
    @get:OutputFile
    val hytaleJarFile: File = project.file("libs/HytaleServer.jar")

    @get:Inject
    abstract val execOps: ExecOperations

    @TaskAction
    fun download() {
        val libsDir = project.file("libs")
        libsDir.mkdirs()

        // Check for credentials file in multiple locations
        val credentialsFile = listOf(
            project.file(".hytale-downloader-credentials.json"),
            File(System.getProperty("user.home"), ".hytale-downloader-credentials.json"),
            project.rootProject.file(".hytale-downloader-credentials.json")
        ).firstOrNull { it.exists() }

        if (credentialsFile == null) {
            throw GradleException("Hytale credentials file not found. Create .hytale-downloader-credentials.json to enable Hytale builds.")
        }

        val tempDir = project.layout.buildDirectory.dir("hytale-download").get().asFile
        tempDir.mkdirs()

        // Determine OS and download appropriate downloader
        val os = System.getProperty("os.name").lowercase()
        val downloaderName = when {
            os.contains("linux") -> "hytale-downloader-linux-amd64"
            os.contains("windows") -> "hytale-downloader-windows-amd64.exe"
            else -> throw GradleException("Unsupported OS: $os")
        }

        val downloaderZip = File(tempDir, "hytale-downloader.zip")
        val downloaderExe = File(tempDir, downloaderName)
        val serverOutputDir = File(tempDir, "server")

        // Download the downloader
        logger.lifecycle("Downloading Hytale server downloader...")
        ant.invokeMethod("get", mapOf(
            "src" to "https://downloader.hytale.com/hytale-downloader.zip",
            "dest" to downloaderZip
        ))

        // Unzip
        project.copy {
            from(project.zipTree(downloaderZip))
            into(tempDir)
        }

        // Make executable on Unix
        if (!os.contains("windows")) {
            execOps.exec {
                commandLine("chmod", "+x", downloaderExe.absolutePath)
            }
        }

        // Copy credentials to temp dir
        credentialsFile.copyTo(File(tempDir, ".hytale-downloader-credentials.json"), overwrite = true)

        // Run downloader
        logger.lifecycle("Downloading Hytale server...")
        execOps.exec {
            workingDir(tempDir)
            commandLine(downloaderExe.absolutePath, "-download-path=${tempDir.absolutePath}")
        }

        // Extract server.zip
        val serverZip = File(tempDir, "server.zip")
        if (!serverZip.exists()) {
            throw GradleException("Hytale server download failed - server.zip not found")
        }

        logger.lifecycle("Extracting Hytale server...")
        serverOutputDir.mkdirs()
        project.copy {
            from(project.zipTree(serverZip))
            into(serverOutputDir)
        }

        // Copy server JAR to libs
        val downloadedJar = File(serverOutputDir, "HytaleServer.jar")
        if (downloadedJar.exists()) {
            downloadedJar.copyTo(hytaleJarFile, overwrite = true)
            logger.lifecycle("Hytale server downloaded to: ${hytaleJarFile.absolutePath}")
        } else {
            // List what was extracted for debugging
            val extractedFiles = serverOutputDir.listFiles()?.map { it.name } ?: emptyList()
            throw GradleException("Hytale server download failed - HytaleServer.jar not found in extracted files: $extractedFiles")
        }
    }
}

val downloadHytaleServer by tasks.registering(DownloadHytaleServerTask::class) {
    group = "hytale"
    description = "Download Hytale server JAR using the official downloader"
    onlyIf { !hytaleJarFile.exists() }
}

// Dependencies - always configured, but tasks use onlyIf for runtime checks
dependencies {
    implementation(project(":common"))

    // Hytale Server (local dependency, provided at runtime)
    if (hytaleJar.exists()) {
        compileOnly(files("libs/HytaleServer.jar"))
    }

    // Test dependencies
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    onlyIf { hytaleJar.exists() }
}

// Use execution-time checks instead of configuration-time
tasks.withType<JavaCompile>().configureEach {
    dependsOn(downloadHytaleServer)
    onlyIf { hytaleJar.exists() }
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    dependsOn(downloadHytaleServer)
    onlyIf { hytaleJar.exists() }
}

tasks.shadowJar {
    dependsOn(downloadHytaleServer)
    onlyIf { hytaleJar.exists() }

    archiveClassifier.set("")
    relocate("com.google.gson", "com.alaydriem.bedrockvoicechat.shaded.gson")
    dependencies {
        include(project(":common"))
        include(dependency("com.google.code.gson:gson"))
    }

    from("LICENSE") {
        rename { "${it}_${archivesBaseName}" }
    }
}

tasks.jar {
    dependsOn(downloadHytaleServer)
    onlyIf { hytaleJar.exists() }
}

tasks.build {
    dependsOn(downloadHytaleServer)
}

// Task to run Hytale server with the plugin for testing
tasks.register<Exec>("runServer") {
    group = "hytale"
    description = "Run Hytale server with plugin for testing"

    dependsOn("shadowJar")
    onlyIf { hytaleJar.exists() }

    val runDir = file("run")
    val pluginsDir = File(runDir, "plugins")

    doFirst {
        // Ensure directories exist
        runDir.mkdirs()
        pluginsDir.mkdirs()

        // Copy plugin JAR to plugins folder
        val shadowJarTask = tasks.named("shadowJar").get()
        val jarFile = shadowJarTask.outputs.files.singleFile
        jarFile.copyTo(File(pluginsDir, jarFile.name), overwrite = true)
        println("Copied plugin JAR to: ${pluginsDir}/${jarFile.name}")

        // Copy HytaleServer.jar to run dir if not present
        val serverJar = File(runDir, "HytaleServer.jar")
        if (!serverJar.exists()) {
            val libsServerJar = file("libs/HytaleServer.jar")
            if (libsServerJar.exists()) {
                libsServerJar.copyTo(serverJar)
                println("Copied HytaleServer.jar to run directory")
            } else {
                throw IllegalStateException("HytaleServer.jar not found in libs/")
            }
        }
    }

    workingDir = runDir

    // Debug mode support
    val javaArgs = mutableListOf("java")
    if (project.hasProperty("debug")) {
        javaArgs.add("-agentlib:jdwp=transport=dt_socket,server=y,suspend=n,address=*:5005")
        println("Debug mode enabled on port 5005")
    }
    javaArgs.addAll(listOf("-jar", "HytaleServer.jar"))

    commandLine(javaArgs)
}

// Log status during configuration
if (!hytaleJar.exists()) {
    logger.warn("HytaleServer.jar not found - Hytale compilation tasks will attempt to download it first")
}
