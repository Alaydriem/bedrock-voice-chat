plugins {
    java
    kotlin("jvm") version "2.0.21"
    id("fabric-loom") version "1.15.1"
}

val minecraftVersion: String by project
val yarnMappings: String by project
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
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_21)
    }
}

dependencies {
    // Minecraft and Fabric
    minecraft("com.mojang:minecraft:$minecraftVersion")
    mappings("net.fabricmc:yarn:$yarnMappings:v2")
    modImplementation("net.fabricmc:fabric-loader:$loaderVersion")
    modImplementation("net.fabricmc.fabric-api:fabric-api:$fabricVersion")

    // Common module (via composite build substitution)
    implementation("com.alaydriem:bedrock-voice-chat-common")
    include("com.alaydriem:bedrock-voice-chat-common")

    // Kotlin
    implementation(kotlin("stdlib"))
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
