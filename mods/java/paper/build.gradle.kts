plugins {
    java
    id("com.gradleup.shadow")
}

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-paper")
}

dependencies {
    // Paper API (provided at runtime)
    compileOnly("io.papermc.paper:paper-api:1.21.4-R0.1-SNAPSHOT")

    // Common module - will be shadowed
    implementation(project(":common"))

    // Test dependencies - MockBukkit for event simulation
    testImplementation("org.mockbukkit.mockbukkit:mockbukkit-v1.21:4.0.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
    testImplementation(kotlin("test"))
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
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
