plugins {
    java
    id("com.gradleup.shadow")
}

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-powernukkitx")
}

repositories {
    maven {
        name = "PowerNukkitX"
        url = uri("https://repo.powernukkitx.org/releases")
    }
    // PowerNukkitX's own dependencies (netty-transport-nethernet,
    // netty-external-signaling, libdatachannel) are published only here.
    maven {
        name = "OpenCollab"
        url = uri("https://repo.opencollab.dev/main/")
    }
}

val pnxVersion = "3.0.6-SNAPSHOT"

dependencies {
    // PowerNukkitX supplies its API, gson, slf4j, snakeyaml and JNA at runtime.
    compileOnly("org.powernukkitx:server:$pnxVersion")

    implementation(project(":common"))

    // The tests mock PowerNukkitX types, so the server has to be on the test
    // runtime classpath as well; compileOnly is not.
    testImplementation("org.powernukkitx:server:$pnxVersion")
    testImplementation("org.junit.jupiter:junit-jupiter:6.1.0")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher:6.1.0")
    testImplementation("org.mockito:mockito-core:5.24.0")
    testImplementation("org.mockito.kotlin:mockito-kotlin:6.4.0")
    testImplementation("org.slf4j:slf4j-api:2.0.9")
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

    // PowerNukkitX's plugin class loader is parent-first and refuses to define a
    // class its parent can see, so a shaded copy of a server library is never
    // used. gson, slf4j and JNA therefore stay out of this jar. Kotlin is not on
    // the server, and is relocated so this plugin loads beside another plugin
    // that ships its own unrelocated Kotlin.
    relocate("kotlin", "com.alaydriem.bedrockvoicechat.shaded.kotlin")
    relocate("kotlinx", "com.alaydriem.bedrockvoicechat.shaded.kotlinx")

    // Coroutines registers its dispatchers through META-INF/services; merging
    // applies the relocation to those entries too.
    mergeServiceFiles()

    dependencies {
        include(project(":common"))
        include(project(":relay-sdk"))
        include(dependency("org.jetbrains.kotlin:kotlin-stdlib"))
        // The -jvm artifact carries BuildersKt; the plain coroutines-core module
        // is a stub that delegates to it.
        include(dependency("org.jetbrains.kotlinx:kotlinx-coroutines-core"))
        include(dependency("org.jetbrains.kotlinx:kotlinx-coroutines-core-jvm"))
        include(dependency("org.jetbrains.kotlinx:kotlinx-coroutines-bom"))
    }
}

tasks.jar {
    enabled = false
}

tasks.build {
    dependsOn(tasks.shadowJar)
}
