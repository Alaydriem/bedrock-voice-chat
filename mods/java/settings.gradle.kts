pluginManagement {
    repositories {
        gradlePluginPortal()
        mavenCentral()
        maven {
            name = "PaperMC"
            url = uri("https://repo.papermc.io/repository/maven-public/")
        }
    }
}

rootProject.name = "bedrock-voice-chat"

include(":common")
// fabric is a separate Gradle project (composite build) due to fabric-loom classpath isolation
// Build it from: cd mods/java/fabric && ./gradlew build
include(":paper")
include(":hytale")
