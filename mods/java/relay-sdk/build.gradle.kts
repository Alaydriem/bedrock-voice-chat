import org.gradle.internal.os.OperatingSystem

plugins {
    `java-library`
    `maven-publish`
    id("com.gradleup.shadow")
}

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-relay-sdk")
}

dependencies {
    // uniffi's Kotlin backend generates JNA bindings and suspending calls.
    api("net.java.dev.jna:jna:5.14.0")
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")

    testImplementation("org.junit.jupiter:junit-jupiter:6.1.0")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher:6.1.0")
}

val bvcRoot = rootProject.projectDir.parentFile.parentFile
val rustBuildMode = if (rootProject.hasProperty("release")) "release" else "debug"

// The SDK cdylib is built from the ROOT workspace, not the server workspace:
// bvc-relay-sdk is a member of the former, and the two have separate targets.
val rustTargetDir = File(bvcRoot, "target/$rustBuildMode")

// JNA resolves a bare library name from the classpath under its own platform
// prefixes, which are not the `native/{os}-{arch}` layout the BVC mod uses for
// its manually loaded library. These are JNA's.
val jnaPrefixes = mapOf(
    "windows-x64" to "win32-x86-64",
    "linux-x64" to "linux-x86-64",
    "linux-arm64" to "linux-aarch64",
    "darwin-arm64" to "darwin-aarch64"
)

tasks.register<Exec>("buildSdkLibrary") {
    group = "native"
    description = "Build the BVC relay SDK cdylib"

    workingDir = bvcRoot

    val args = mutableListOf("cargo", "build", "-p", "bvc-relay-sdk")
    if (rootProject.hasProperty("release")) {
        args.add("--release")
    }
    commandLine(args)
}

tasks.register<Exec>("buildEchoPeer") {
    group = "verification"
    description = "Build the echo peer the smoke test dials"

    workingDir = bvcRoot
    commandLine("cargo", "build", "-p", "bvc-peer-echo")
}

tasks.register<Copy>("copyNativeWindows") {
    group = "native"
    description = "Copy the Windows x64 SDK library into JNA's resource prefix"

    dependsOn("buildSdkLibrary")
    from(rustTargetDir) { include("bvc_relay_sdk.dll") }
    into(layout.projectDirectory.dir("src/main/resources/${jnaPrefixes["windows-x64"]}"))
}

tasks.register<Copy>("copyNativeLinuxX64") {
    group = "native"
    from(File(bvcRoot, "target/x86_64-unknown-linux-gnu/release")) {
        include("libbvc_relay_sdk.so")
    }
    into(layout.projectDirectory.dir("src/main/resources/${jnaPrefixes["linux-x64"]}"))
}

tasks.register<Copy>("copyNativeLinuxArm64") {
    group = "native"
    from(File(bvcRoot, "target/aarch64-unknown-linux-gnu/release")) {
        include("libbvc_relay_sdk.so")
    }
    into(layout.projectDirectory.dir("src/main/resources/${jnaPrefixes["linux-arm64"]}"))
}

tasks.register<Copy>("copyNativeDarwinArm64") {
    group = "native"
    from(File(bvcRoot, "target/aarch64-apple-darwin/release")) {
        include("libbvc_relay_sdk.dylib")
    }
    into(layout.projectDirectory.dir("src/main/resources/${jnaPrefixes["darwin-arm64"]}"))
}

tasks.register("copyNativeLibraries") {
    group = "native"
    description = "Copy every available SDK library into resources"
    dependsOn(
        "copyNativeWindows",
        "copyNativeLinuxX64",
        "copyNativeLinuxArm64",
        "copyNativeDarwinArm64"
    )
}

// Regenerates the checked-in bindings. Run after changing the Rust surface; the
// output is committed so building this module needs no Rust toolchain.
tasks.register<Exec>("generateBindings") {
    group = "native"
    description = "Regenerate the Kotlin bindings from the cdylib"

    dependsOn("buildSdkLibrary")
    workingDir = bvcRoot

    val libName = when {
        OperatingSystem.current().isWindows -> "bvc_relay_sdk.dll"
        OperatingSystem.current().isMacOsX -> "libbvc_relay_sdk.dylib"
        else -> "libbvc_relay_sdk.so"
    }

    commandLine(
        "cargo", "run", "-p", "bvc-relay-sdk", "--bin", "uniffi-bindgen", "--",
        "generate", "--library", "target/$rustBuildMode/$libName",
        "--language", "kotlin",
        "--out-dir", "mods/java/relay-sdk/src/main/kotlin",
        "--no-format"
    )
}

tasks.named("processResources") {
    mustRunAfter(
        "copyNativeWindows",
        "copyNativeLinuxX64",
        "copyNativeLinuxArm64",
        "copyNativeDarwinArm64"
    )
}

// Two plugins each bundling their own JNA fail at load with "Native Library
// jnidispatch already loaded in another classloader". Relocating in every jar
// that bundles it is what keeps them able to share a server.
tasks.shadowJar {
    archiveClassifier.set("")
    relocate("com.sun.jna", "com.alaydriem.bedrockvoicechat.shaded.jna")

    dependencies {
        include(dependency("net.java.dev.jna:jna"))
        include(dependency("org.jetbrains.kotlinx:kotlinx-coroutines-core"))
        include(dependency("org.jetbrains.kotlinx:kotlinx-coroutines-bom"))
        include(dependency("org.jetbrains.kotlin:kotlin-stdlib"))
    }
}

tasks.test {
    useJUnitPlatform()
    dependsOn("buildSdkLibrary", "buildEchoPeer")

    // The tests run against the freshly built cdylib rather than whatever is in
    // resources, so a stale copy cannot make a broken build look green.
    systemProperty("jna.library.path", rustTargetDir.absolutePath)

    val echoName = if (OperatingSystem.current().isWindows) {
        "bvc-peer-echo.exe"
    } else {
        "bvc-peer-echo"
    }
    systemProperty("bvc.echoPeer", File(rustTargetDir, echoName).absolutePath)
}

publishing {
    publications {
        create<MavenPublication>("relaySdk") {
            artifactId = "bedrock-voice-chat-relay-sdk"
            // The shadow jar rather than the thin one: JNA is relocated in it,
            // and a consumer resolving the thin jar would pull an unrelocated
            // JNA back in transitively — which is the collision this avoids.
            artifact(tasks.shadowJar)
        }
    }
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/alaydriem/bedrock-voice-chat")
            credentials {
                username = System.getenv("GITHUB_ACTOR")
                password = System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
