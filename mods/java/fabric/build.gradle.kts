plugins {
    id("fabric-loom")
}

val minecraftVersion: String by project
val yarnMappings: String by project
val loaderVersion: String by project
val fabricVersion: String by project
val archivesBaseName: String by project

base {
    archivesName.set(archivesBaseName)
}

dependencies {
    // Minecraft and Fabric
    minecraft("com.mojang:minecraft:$minecraftVersion")
    mappings("net.fabricmc:yarn:$yarnMappings:v2")
    modImplementation("net.fabricmc:fabric-loader:$loaderVersion")
    modImplementation("net.fabricmc.fabric-api:fabric-api:$fabricVersion")

    // Common module
    implementation(project(":common"))
    include(project(":common"))
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
