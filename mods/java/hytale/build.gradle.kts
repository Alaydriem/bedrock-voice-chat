import org.gradle.api.DefaultTask
import org.gradle.api.tasks.TaskAction
import org.gradle.api.tasks.OutputFile
import com.google.gson.Gson
import com.google.gson.JsonObject
import java.net.URI
import java.net.URLEncoder
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.security.MessageDigest

plugins {
    java
    id("com.gradleup.shadow")
}

val archivesBaseName: String by project
val hytaleJar = file("build/hytale-download/server/Server/HytaleServer.jar")

base {
    archivesName.set("$archivesBaseName-hytale")
}

// Task to refresh Hytale OAuth tokens
abstract class RefreshHytaleTokenTask : DefaultTask() {
    private val httpClient = HttpClient.newBuilder().followRedirects(HttpClient.Redirect.NORMAL).build()
    private val gson = Gson()

    @TaskAction
    fun refresh() {
        // Find credentials file
        val credentialsFile = listOf(
            project.file(".hytale-downloader-credentials.json"),
            project.rootProject.file(".hytale-downloader-credentials.json")
        ).firstOrNull { it.exists() }
            ?: throw GradleException("Hytale credentials file not found. Create .hytale-downloader-credentials.json first.")

        logger.lifecycle("Reading credentials from: ${credentialsFile.absolutePath}")

        // Parse existing credentials
        val credentials = gson.fromJson(credentialsFile.readText(), JsonObject::class.java)
        val refreshToken = credentials.get("refresh_token")?.asString
            ?: throw GradleException("No refresh_token found in credentials file")

        // Call OAuth2 token endpoint
        logger.lifecycle("Refreshing OAuth token...")
        val formData = "grant_type=refresh_token&refresh_token=${URLEncoder.encode(refreshToken, "UTF-8")}&client_id=hytale-server"
        val request = HttpRequest.newBuilder()
            .uri(URI.create("https://oauth.accounts.hytale.com/oauth2/token"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .POST(HttpRequest.BodyPublishers.ofString(formData))
            .build()

        val response = httpClient.send(request, HttpResponse.BodyHandlers.ofString())
        val responseJson = gson.fromJson(response.body(), JsonObject::class.java)

        // Check for errors
        if (responseJson.has("error")) {
            val error = responseJson.get("error_description")?.asString
                ?: responseJson.get("error")?.asString
                ?: "Unknown error"
            throw GradleException("Token refresh failed: $error")
        }

        // Extract new tokens
        val newAccessToken = responseJson.get("access_token")?.asString
            ?: throw GradleException("No access_token in response")
        val newRefreshToken = responseJson.get("refresh_token")?.asString ?: refreshToken
        val expiresIn = responseJson.get("expires_in")?.asInt ?: 3600
        val idToken = responseJson.get("id_token")?.asString
        val scope = responseJson.get("scope")?.asString
        val tokenType = responseJson.get("token_type")?.asString

        // Build updated credentials (preserve original format)
        val updatedCredentials = JsonObject().apply {
            addProperty("access_token", newAccessToken)
            addProperty("expires_in", expiresIn)
            if (idToken != null) addProperty("id_token", idToken)
            addProperty("refresh_token", newRefreshToken)
            if (scope != null) addProperty("scope", scope)
            if (tokenType != null) addProperty("token_type", tokenType)
        }

        // Write back to file
        credentialsFile.writeText(gson.toJson(updatedCredentials))

        logger.lifecycle("Token refreshed successfully!")
        if (newRefreshToken != refreshToken) {
            logger.lifecycle("Note: Refresh token was rotated by the server")
        }
        logger.lifecycle("Token expires in $expiresIn seconds")
    }
}

val refreshHytaleToken by tasks.registering(RefreshHytaleTokenTask::class) {
    group = "hytale"
    description = "Refresh Hytale OAuth access token using the refresh token"
}

// Task to download Hytale server using direct API calls
abstract class DownloadHytaleServerTask : DefaultTask() {
    @get:OutputFile
    val hytaleJarFile: File = project.file("build/hytale-download/server/Server/HytaleServer.jar")

    private val httpClient = HttpClient.newBuilder().followRedirects(HttpClient.Redirect.NORMAL).build()
    private val gson = Gson()

    @TaskAction
    fun download() {
        // Check for credentials file in multiple locations
        val credentialsFile = listOf(
            project.file(".hytale-downloader-credentials.json"),
            project.rootProject.file(".hytale-downloader-credentials.json")
        ).firstOrNull { it.exists() }

        if (credentialsFile == null) {
            throw GradleException("Hytale credentials file not found. Create .hytale-downloader-credentials.json to enable Hytale builds.")
        }

        // Parse access token from credentials
        val credentials = gson.fromJson(credentialsFile.readText(), JsonObject::class.java)
        val accessToken = credentials.get("access_token")?.asString
            ?: throw GradleException("No access_token found in credentials file")

        val tempDir = project.layout.buildDirectory.dir("hytale-download").get().asFile
        tempDir.mkdirs()

        // Step 1: Get release info signed URL
        logger.lifecycle("Fetching release info...")
        val releaseSignedUrl = fetchSignedUrl(
            "https://account-data.hytale.com/game-assets/version/release.json",
            accessToken
        )

        // Step 2: Fetch release info (version, download_url, sha256)
        val releaseInfo = fetchJson(releaseSignedUrl)
        val releaseJson = gson.fromJson(releaseInfo, JsonObject::class.java)
        val downloadPath = releaseJson.get("download_url")?.asString
            ?: throw GradleException("No download_url in release info")
        val expectedSha256 = releaseJson.get("sha256")?.asString
            ?: throw GradleException("No sha256 in release info")
        val version = releaseJson.get("version")?.asString ?: "unknown"
        logger.lifecycle("Found Hytale server version: $version")

        // Step 3: Get download signed URL
        logger.lifecycle("Fetching download URL...")
        val downloadSignedUrl = fetchSignedUrl(
            "https://account-data.hytale.com/game-assets/$downloadPath",
            accessToken
        )

        // Step 4: Download server.zip
        val serverZip = File(tempDir, "server.zip")
        logger.lifecycle("Downloading Hytale server...")
        downloadFile(downloadSignedUrl, serverZip)

        // Step 5: Verify SHA256
        logger.lifecycle("Verifying checksum...")
        val actualSha256 = sha256(serverZip)
        if (!actualSha256.equals(expectedSha256, ignoreCase = true)) {
            throw GradleException("SHA256 mismatch! Expected: $expectedSha256, Got: $actualSha256")
        }
        logger.lifecycle("Checksum verified.")

        // Step 6: Extract server.zip
        logger.lifecycle("Extracting Hytale server...")
        val serverOutputDir = File(tempDir, "server")
        serverOutputDir.mkdirs()
        project.copy {
            from(project.zipTree(serverZip))
            into(serverOutputDir)
        }

        // Step 7: Verify JAR exists (JAR is inside Server subdirectory)
        if (hytaleJarFile.exists()) {
            logger.lifecycle("Hytale server downloaded to: ${hytaleJarFile.absolutePath}")
        } else {
            val extractedFiles = serverOutputDir.listFiles()?.map { it.name } ?: emptyList()
            val serverDirFiles = File(serverOutputDir, "Server").listFiles()?.map { it.name } ?: emptyList()
            throw GradleException("HytaleServer.jar not found. Root: $extractedFiles, Server/: $serverDirFiles")
        }
    }

    private fun fetchSignedUrl(endpoint: String, token: String): String {
        val request = HttpRequest.newBuilder()
            .uri(URI.create(endpoint))
            .header("Authorization", "Bearer $token")
            .GET()
            .build()
        val response = httpClient.send(request, HttpResponse.BodyHandlers.ofString())
        if (response.statusCode() != 200) {
            throw GradleException("Failed to fetch $endpoint: HTTP ${response.statusCode()} - ${response.body()}")
        }
        val json = gson.fromJson(response.body(), JsonObject::class.java)
        return json.get("url")?.asString
            ?: throw GradleException("No 'url' field in response from $endpoint")
    }

    private fun fetchJson(url: String): String {
        val request = HttpRequest.newBuilder()
            .uri(URI.create(url))
            .GET()
            .build()
        val response = httpClient.send(request, HttpResponse.BodyHandlers.ofString())
        if (response.statusCode() != 200) {
            throw GradleException("Failed to fetch $url: HTTP ${response.statusCode()}")
        }
        return response.body()
    }

    private fun downloadFile(url: String, dest: File) {
        val request = HttpRequest.newBuilder()
            .uri(URI.create(url))
            .GET()
            .build()
        val response = httpClient.send(request, HttpResponse.BodyHandlers.ofInputStream())
        if (response.statusCode() != 200) {
            throw GradleException("Failed to download $url: HTTP ${response.statusCode()}")
        }
        dest.outputStream().use { out ->
            response.body().copyTo(out)
        }
    }

    private fun sha256(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(8192)
            var bytesRead: Int
            while (input.read(buffer).also { bytesRead = it } != -1) {
                digest.update(buffer, 0, bytesRead)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }
}

val downloadHytaleServer by tasks.registering(DownloadHytaleServerTask::class) {
    group = "hytale"
    description = "Download Hytale server JAR using direct API calls"
    onlyIf { !hytaleJarFile.exists() }
}

// Dependencies - always configured, but tasks use onlyIf for runtime checks
dependencies {
    implementation(project(":common"))

    // Hytale Server (local dependency, provided at runtime)
    // Always add to classpath - downloadHytaleServer task ensures JAR exists before compile
    compileOnly(files("build/hytale-download/server/Server/HytaleServer.jar"))

    // SLF4J - not provided by Hytale's plugin classloader, must be bundled
    implementation("org.slf4j:slf4j-api:2.0.9")
    implementation("org.slf4j:slf4j-simple:2.0.9")

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
    mergeServiceFiles()
    relocate("com.google.gson", "com.alaydriem.bedrockvoicechat.shaded.gson")
    // Note: SLF4J not relocated - ServiceLoader doesn't work well with relocation
    dependencies {
        include(project(":common"))
        include(dependency("com.google.code.gson:gson:.*"))
        // Include all Kotlin runtime dependencies (group:name:version pattern)
        include(dependency("org.jetbrains.kotlin:kotlin-stdlib:.*"))
        include(dependency("org.jetbrains:annotations:.*"))
        // SLF4J - not provided by Hytale's plugin classloader
        include(dependency("org.slf4j:slf4j-api:.*"))
        include(dependency("org.slf4j:slf4j-simple:.*"))
        // JNA for native library loading (FFI with embedded BVC server)
        include(dependency("net.java.dev.jna:jna:.*"))
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
            val downloadedServerJar = file("build/hytale-download/server/Server/HytaleServer.jar")
            if (downloadedServerJar.exists()) {
                downloadedServerJar.copyTo(serverJar)
                println("Copied HytaleServer.jar to run directory")
            } else {
                throw IllegalStateException("HytaleServer.jar not found in build/hytale-download/server/Server/")
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
