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

rootProject.name = "bedrock-voice-chat-fabric"

// Include parent build to access common module
includeBuild("..") {
    dependencySubstitution {
        substitute(module("com.alaydriem:bedrock-voice-chat-common")).using(project(":common"))
    }
}
