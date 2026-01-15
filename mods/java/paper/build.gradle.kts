plugins {
    java
}

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-paper")
}

dependencies {
    // Paper API
    compileOnly("io.papermc.paper:paper-api:1.21.4-R0.1-SNAPSHOT")

    // Common module
    implementation(project(":common"))
}

tasks.processResources {
    inputs.property("version", project.version)

    filesMatching("plugin.yml") {
        expand("version" to project.version)
    }
}

tasks.jar {
    // Include common module in the jar
    from(project(":common").sourceSets.main.get().output)

    from("LICENSE") {
        rename { "${it}_${archivesBaseName}" }
    }
}
