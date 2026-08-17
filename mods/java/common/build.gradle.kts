// Fully qualifying this below does not work: the `java` extension the Kotlin DSL
// adds shadows the `java` package inside the script body.
import java.security.MessageDigest

plugins {
    `java-library`
}

val archivesBaseName: String by project
val voicechatApiVersion: String by project

base {
    archivesName.set("$archivesBaseName-common")
}

repositories {
    maven {
        name = "OpenCollab"
        url = uri("https://repo.opencollab.dev/main/")
    }
    maven {
        name = "Maxhenkel"
        url = uri("https://maven.maxhenkel.de/repository/public")
    }
}

dependencies {
    // Gson for JSON serialization
    api("com.google.code.gson:gson:2.10.1")

    // JNA for native library loading (FFI with Rust BVC server)
    api("net.java.dev.jna:jna:5.14.0")

    // SLF4J for logging (provided by platform implementations)
    compileOnly("org.slf4j:slf4j-api:2.0.9")

    // Floodgate API for Geyser/Bedrock player detection (optional at runtime)
    compileOnly("org.geysermc.floodgate:api:2.2.5-SNAPSHOT")

    // Simple Voice Chat, for the bridge. compileOnly rather than reflective like
    // Floodgate, because the bridge implements VoicechatPlugin and receives
    // callbacks — reflection can call an API but cannot be called back by one.
    // Classes referencing it load only after SvcAvailability confirms it is there.
    compileOnly("de.maxhenkel.voicechat:voicechat-api:$voicechatApiVersion")
    testImplementation("de.maxhenkel.voicechat:voicechat-api:$voicechatApiVersion")

    // The peer link the bridge sends over. `api` rather than `implementation`
    // because SdkFrame appears in signatures the platform modules call.
    api(project(":relay-sdk"))

    // slf4j is compileOnly above because platforms provide it; tests still load
    // classes that hold a logger, so it has to be on the test classpath.
    testImplementation("org.slf4j:slf4j-api:2.0.9")
    testImplementation("org.junit.jupiter:junit-jupiter:6.1.0")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher:6.1.0")
}

tasks.test {
    useJUnitPlatform()
}

// Native library bundling configuration
// Libraries are expected at: {projectRoot}/server/target/{debug|release}/
// (server workspace target directory)
// and copied to: src/main/resources/native/{os}-{arch}/
// Use -Prelease for release builds (default is debug for faster iteration)

// Navigate from mods/java/ to bvc/ then to server/target/{mode}/
val bvcRoot = rootProject.projectDir.parentFile.parentFile
val rustBuildMode = if (rootProject.hasProperty("release")) "release" else "debug"
val rustTargetDir = File(bvcRoot, "server/target/$rustBuildMode")

// Task to copy Windows x64 native library
tasks.register<Copy>("copyNativeWindows") {
    group = "native"
    description = "Copy Windows x64 native library to resources"

    from(rustTargetDir) {
        include("bvc_server_lib.dll")
    }
    into(layout.projectDirectory.dir("src/main/resources/native/windows-x64"))

    doFirst {
        val dllFile = File(rustTargetDir, "bvc_server_lib.dll")
        if (!dllFile.exists()) {
            logger.warn("Native library not found at ${dllFile.absolutePath}")
            logger.warn("Build with: cd server/server && cargo build --release --lib")
        }
    }
}

// The fat jar bundles every platform; the skinny jar carries only the manifest and
// resolves at runtime. This flag is the only difference between the two artifacts.
val bundleNatives = rootProject.hasProperty("bundled")

val nativeResourceDir = layout.projectDirectory.dir("src/main/resources/native").asFile
val generatedManifestDir = layout.buildDirectory.dir("generated/nativeManifest")

// The release the manifest pins. CI passes the real tag; a local build gets "dev",
// whose manifest resolves nothing and fails by name rather than by fetching from
// a release that has nothing to do with this build.
val nativeRelease = (project.findProperty("nativeRelease") as String?) ?: "dev"
val nativeRepo = (project.findProperty("nativeRepo") as String?) ?: "alaydriem/bedrock-voice-chat"

// One generator for local builds and for CI, so the digests a jar pins are always
// of the files that build actually saw. Two implementations of this could disagree,
// and the failure would be a jar that refuses every library it downloads.
tasks.register("generateNativeManifest") {
    group = "native"
    description = "Writes native-manifest.json from the native libraries present"

    // The copies write the very directory this hashes, so in a fat build they have
    // to finish first. Without this the manifest pins whatever was on disk from an
    // earlier build, and every download it describes would then fail verification.
    if (bundleNatives) {
        dependsOn("copyNativeLibraries")
    }

    inputs.dir(nativeResourceDir).optional(true)
    inputs.property("release", nativeRelease)
    inputs.property("repo", nativeRepo)
    outputs.dir(generatedManifestDir)

    doLast {
        val libraries = linkedMapOf<String, MutableMap<String, Map<String, String>>>()

        if (nativeResourceDir.isDirectory) {
            for (platformDir in nativeResourceDir.listFiles().orEmpty().sortedBy { it.name }) {
                if (!platformDir.isDirectory) continue
                for (libFile in platformDir.listFiles().orEmpty().sortedBy { it.name }) {
                    if (!libFile.isFile) continue

                    val stem = libFile.name.substringBeforeLast('.')
                    val extension = libFile.name.substringAfterLast('.')
                    val library = stem.removePrefix("lib")
                    val digest = MessageDigest.getInstance("SHA-256")
                        .digest(libFile.readBytes())
                        .joinToString("") { byte -> "%02x".format(byte) }

                    libraries.getOrPut(library) { linkedMapOf() }[platformDir.name] = mapOf(
                        "asset" to "$stem-${platformDir.name}.$extension",
                        "sha256" to digest
                    )
                }
            }
        }

        val manifest = mapOf(
            "release" to nativeRelease,
            "base_url" to "https://github.com/$nativeRepo/releases/download/$nativeRelease",
            "libraries" to libraries
        )

        val outDir = generatedManifestDir.get().asFile
        outDir.mkdirs()
        File(outDir, "native-manifest.json").writeText(
            groovy.json.JsonBuilder(manifest).toPrettyString() + "\n"
        )
    }
}

sourceSets.named("main") {
    resources.srcDir(generatedManifestDir)
}

tasks.named<ProcessResources>("processResources") {
    dependsOn("generateNativeManifest")

    if (bundleNatives) {
        dependsOn("copyNativeLibraries")
    } else {
        // Excluded at packaging rather than checked on disk, so a skinny build is
        // correct on a working tree that still has libraries from an earlier fat
        // build. Those files are gitignored and routinely present.
        exclude("native/**")
    }
}

// A skinny jar containing a native library is a silent regression: it would work
// perfectly and ship the bytes this exists to remove.
tasks.register("verifyNoBundledNatives") {
    group = "verification"
    description = "Fails if a skinny build packaged a native library"

    dependsOn("processResources")

    doLast {
        if (bundleNatives) {
            return@doLast
        }
        val packaged = File(layout.buildDirectory.get().asFile, "resources/main/native")
        val found = packaged.walkTopDown().filter { it.isFile }.toList()
        if (found.isNotEmpty()) {
            throw GradleException(
                "Skinny build packaged ${found.size} native libraries: ${found.joinToString { it.name }}"
            )
        }
    }
}

tasks.named("check") {
    dependsOn("verifyNoBundledNatives")
}

// Task to copy Linux x64 native library (native build)
tasks.register<Copy>("copyNativeLinuxX64") {
    group = "native"
    description = "Copy Linux x64 native library to resources"

    from(rustTargetDir) {
        include("libbvc_server_lib.so")
    }
    into(layout.projectDirectory.dir("src/main/resources/native/linux-x64"))
}

// Task to copy Linux x64 native library (cross-compiled)
tasks.register<Copy>("copyNativeLinuxX64Cross") {
    group = "native"
    description = "Copy Linux x64 native library from cross-compilation target"

    from(File(bvcRoot, "server/target/x86_64-unknown-linux-gnu/release")) {
        include("libbvc_server_lib.so")
    }
    into(layout.projectDirectory.dir("src/main/resources/native/linux-x64"))
}

// Task to copy Linux ARM64 native library (cross-compiled)
tasks.register<Copy>("copyNativeLinuxArm64") {
    group = "native"
    description = "Copy Linux ARM64 native library to resources"

    from(File(bvcRoot, "server/target/aarch64-unknown-linux-gnu/release")) {
        include("libbvc_server_lib.so")
    }
    into(layout.projectDirectory.dir("src/main/resources/native/linux-arm64"))
}

// Task to copy macOS ARM64 native library (native build on Apple Silicon or cross-compiled)
tasks.register<Copy>("copyNativeDarwinArm64") {
    group = "native"
    description = "Copy macOS ARM64 (Apple Silicon) native library to resources"

    from(File(bvcRoot, "server/target/aarch64-apple-darwin/release")) {
        include("libbvc_server_lib.dylib")
    }
    into(layout.projectDirectory.dir("src/main/resources/native/darwin-arm64"))
}

// Convenience task to copy all available native libraries
// Supported architectures: Windows x64, Linux x64, Linux ARM64, macOS ARM64
tasks.register("copyNativeLibraries") {
    group = "native"
    description = "Copy all available native libraries to resources"
    dependsOn(
        "copyNativeWindows",
        "copyNativeLinuxX64",
        "copyNativeLinuxArm64",
        "copyNativeDarwinArm64"
    )
}

/*
 * Cross-Compilation Setup for Native Libraries
 * =============================================
 *
 * The native BVC server library needs to be compiled for each target platform.
 * Build from the server directory: bvc/server/
 *
 * Supported architectures:
 *   - Windows x64
 *   - Linux x64
 *   - Linux ARM64 (aarch64)
 *   - macOS ARM64 (Apple Silicon)
 *
 * NATIVE BUILDS (run on target platform):
 *
 * Windows (x64):
 *   cargo build --release --lib
 *   ./gradlew :common:copyNativeWindows
 *
 * Linux (x64):
 *   cargo build --release --lib
 *   ./gradlew :common:copyNativeLinuxX64
 *
 * macOS (ARM64 - Apple Silicon):
 *   cargo build --release --lib
 *   ./gradlew :common:copyNativeDarwinArm64
 *
 * CROSS-COMPILATION (requires toolchains):
 *
 * Linux x64 from Windows/macOS:
 *   rustup target add x86_64-unknown-linux-gnu
 *   cargo build --release --lib --target x86_64-unknown-linux-gnu
 *   ./gradlew :common:copyNativeLinuxX64Cross
 *
 * Linux ARM64 (e.g., Raspberry Pi, AWS Graviton):
 *   rustup target add aarch64-unknown-linux-gnu
 *   cargo build --release --lib --target aarch64-unknown-linux-gnu
 *   ./gradlew :common:copyNativeLinuxArm64
 *
 * macOS ARM64 (cross-compile from Linux):
 *   rustup target add aarch64-apple-darwin
 *   cargo build --release --lib --target aarch64-apple-darwin
 *   ./gradlew :common:copyNativeDarwinArm64
 *
 * LIBRARY PATHS (expected by BvcNative.kt):
 *   Windows x64:    native/windows-x64/bvc_server_lib.dll
 *   Linux x64:      native/linux-x64/libbvc_server_lib.so
 *   Linux ARM64:    native/linux-arm64/libbvc_server_lib.so
 *   macOS ARM64:    native/darwin-arm64/libbvc_server_lib.dylib
 *
 * Copy all at once (skips missing):
 *   ./gradlew :common:copyNativeLibraries
 */
