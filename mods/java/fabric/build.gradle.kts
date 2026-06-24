plugins {
    java
    kotlin("jvm") version "2.3.20"
    id("net.fabricmc.fabric-loom") version "1.17.12"
}

val minecraftVersion: String by project
val loaderVersion: String by project
val fabricVersion: String by project
val archivesBaseName: String by project
val modVersion: String by project
val mavenGroup: String by project

group = mavenGroup
version = modVersion

base {
    archivesName.set(archivesBaseName)
}

repositories {
    mavenCentral()
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(25))
    }
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_25)
    }
}

dependencies {
    // Minecraft and Fabric
    // 26.1+ ships the unobfuscated game; the non-remap loom plugin needs no mappings dependency
    minecraft("com.mojang:minecraft:$minecraftVersion")
    implementation("net.fabricmc:fabric-loader:$loaderVersion")
    implementation("net.fabricmc.fabric-api:fabric-api:$fabricVersion")

    // Common module (via composite build substitution)
    implementation("com.alaydriem:bedrock-voice-chat-common")
    include("com.alaydriem:bedrock-voice-chat-common")

    // Kotlin (include to bundle in JAR)
    implementation(kotlin("stdlib"))
    include("org.jetbrains.kotlin:kotlin-stdlib:2.3.20")
}

loom {
    // Server-side only mod
    runConfigs.configureEach {
        ideConfigGenerated(true)
    }
}

tasks.processResources {
    inputs.property("version", project.version)

    filesMatching("fabric.mod.json") {
        expand("version" to project.version)
    }
}

tasks.jar {
    from("LICENSE") {
        rename { "${it}_${archivesBaseName}" }
    }
}
