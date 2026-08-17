plugins {
    java
    id("com.gradleup.shadow")
}

val archivesBaseName: String by project
val voicechatApiVersion: String by project

base {
    archivesName.set("$archivesBaseName-paper")
}

dependencies {
    // Paper API (provided at runtime)
    compileOnly("io.papermc.paper:paper-api:26.2.build.+")

    // Common module - will be shadowed
    implementation(project(":common"))

    // Simple Voice Chat, for the optional bridge. compileOnly: the server that
    // runs SVC provides it, and a server without SVC loads no class that names it.
    compileOnly("de.maxhenkel.voicechat:voicechat-api:$voicechatApiVersion")

    // Test dependencies - MockBukkit simulates a Paper 26.1.2 server; it ships no paper-api,
    // so a matched paper-api is provided on the test classpath (one minor behind the 26.2 ship target)
    testImplementation("io.papermc.paper:paper-api:26.1.2.build.+")
    testImplementation("org.mockbukkit.mockbukkit:mockbukkit-v26.1.2:4.113.2")
    testImplementation("org.junit.jupiter:junit-jupiter:6.1.0")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher:6.1.0")
}

tasks.test {
    useJUnitPlatform()
}

tasks.processResources {
    inputs.property("version", project.version)

    filesMatching("plugin.yml") {
        expand("version" to project.version)
    }
}

tasks.shadowJar {
    archiveClassifier.set("")

    // Relocate Gson to avoid conflicts with server-provided version
    relocate("com.google.gson", "com.alaydriem.bedrockvoicechat.shaded.gson")

    // JNA is bundled below. Two plugins each bundling an unrelocated copy fail
    // at load with "Native Library jnidispatch already loaded in another
    // classloader", which is what a bridge plugin sharing this server would hit.
    relocate("com.sun.jna", "com.alaydriem.bedrockvoicechat.shaded.jna")

    // Include common module and its dependencies
    dependencies {
        include(project(":common"))
        // The relay SDK and its generated bindings, for the SVC bridge's peer link.
        // Without it the bridge classes reference a uniffi package that is not in
        // the jar, and the plugin fails at load rather than at build.
        include(project(":relay-sdk"))
        include(dependency("com.google.code.gson:gson"))
        include(dependency("org.jetbrains.kotlin:kotlin-stdlib"))
        // The generated bindings expose suspending calls.
        include(dependency("org.jetbrains.kotlinx:kotlinx-coroutines-core"))
        include(dependency("org.jetbrains.kotlinx:kotlinx-coroutines-bom"))
        // JNA for native library loading (FFI with embedded BVC server)
        include(dependency("net.java.dev.jna:jna"))
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
