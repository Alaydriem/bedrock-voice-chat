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
val voicechatApiVersion: String by project

group = mavenGroup
version = modVersion

base {
    archivesName.set(archivesBaseName)
}

repositories {
    mavenCentral()
    // Simple Voice Chat's plugin API, for the optional SVC bridge.
    maven {
        name = "Maxhenkel"
        url = uri("https://maven.maxhenkel.de/repository/public")
    }
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

    // Simple Voice Chat, for the optional bridge. compileOnly: the server that runs
    // SVC provides it, and a server without SVC loads no class that names it.
    compileOnly("de.maxhenkel.voicechat:voicechat-api:$voicechatApiVersion")

    // The relay SDK and its generated bindings, for the SVC bridge's peer link.
    // Bundled explicitly: it arrives transitively from :common, but `include` is
    // what puts a jar inside the mod, and without it the bridge fails at load.
    implementation("com.alaydriem:bedrock-voice-chat-relay-sdk")
    include("com.alaydriem:bedrock-voice-chat-relay-sdk")

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
