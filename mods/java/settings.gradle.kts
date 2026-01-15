pluginManagement {
    repositories {
        maven {
            name = "Fabric"
            url = uri("https://maven.fabricmc.net/")
        }
        gradlePluginPortal()
        mavenCentral()
    }
}

rootProject.name = "bedrock-voice-chat"

include(":common")
include(":fabric")
include(":paper")
include(":hytale")
