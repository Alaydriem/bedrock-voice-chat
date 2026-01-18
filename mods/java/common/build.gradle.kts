plugins {
    `java-library`
}

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-common")
}

dependencies {
    // Gson for JSON serialization
    api("com.google.code.gson:gson:2.10.1")

    // JNA for native library loading (FFI with Rust BVC server)
    api("net.java.dev.jna:jna:5.14.0")

    // SLF4J for logging (provided by platform implementations)
    compileOnly("org.slf4j:slf4j-api:2.0.9")
}

// Native library bundling configuration
// Libraries are expected at: {projectRoot}/server/target/release/
// (server workspace target directory)
// and copied to: src/main/resources/native/{os}-{arch}/

// Navigate from mods/java/ to bvc/ then to server/target/release/
val bvcRoot = rootProject.projectDir.parentFile.parentFile
val rustTargetDir = File(bvcRoot, "server/target/release")

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

// Task to copy macOS x64 native library (native build on Intel Mac)
tasks.register<Copy>("copyNativeDarwinX64") {
    group = "native"
    description = "Copy macOS x64 native library to resources"

    from(rustTargetDir) {
        include("libbvc_server_lib.dylib")
    }
    into(layout.projectDirectory.dir("src/main/resources/native/darwin-x64"))
}

// Task to copy macOS x64 native library (cross-compiled)
tasks.register<Copy>("copyNativeDarwinX64Cross") {
    group = "native"
    description = "Copy macOS x64 native library from cross-compilation target"

    from(File(bvcRoot, "server/target/x86_64-apple-darwin/release")) {
        include("libbvc_server_lib.dylib")
    }
    into(layout.projectDirectory.dir("src/main/resources/native/darwin-x64"))
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
tasks.register("copyNativeLibraries") {
    group = "native"
    description = "Copy all available native libraries to resources"
    dependsOn(
        "copyNativeWindows",
        "copyNativeLinuxX64",
        "copyNativeLinuxArm64",
        "copyNativeDarwinX64",
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
 * macOS (x64 - Intel):
 *   cargo build --release --lib
 *   ./gradlew :common:copyNativeDarwinX64
 *
 * macOS (arm64 - Apple Silicon):
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
 * macOS x64 from ARM Mac:
 *   rustup target add x86_64-apple-darwin
 *   cargo build --release --lib --target x86_64-apple-darwin
 *   ./gradlew :common:copyNativeDarwinX64Cross
 *
 * macOS ARM64 from Intel Mac:
 *   rustup target add aarch64-apple-darwin
 *   cargo build --release --lib --target aarch64-apple-darwin
 *   ./gradlew :common:copyNativeDarwinArm64
 *
 * LIBRARY PATHS (expected by BvcNative.kt):
 *   Windows x64:    native/windows-x64/bvc_server_lib.dll
 *   Linux x64:      native/linux-x64/libbvc_server_lib.so
 *   Linux ARM64:    native/linux-arm64/libbvc_server_lib.so
 *   macOS x64:      native/darwin-x64/libbvc_server_lib.dylib
 *   macOS ARM64:    native/darwin-arm64/libbvc_server_lib.dylib
 *
 * Copy all at once (skips missing):
 *   ./gradlew :common:copyNativeLibraries
 */
