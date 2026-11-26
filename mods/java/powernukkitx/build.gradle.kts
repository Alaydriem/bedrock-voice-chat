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
        // Asked for PowerNukkitX alone, so an outage there stalls nothing else.
        content { includeGroupByRegex("org\\.powernukkitx(\\..*)?") }
    }
    // PowerNukkitX's own dependencies (netty-transport-nethernet,
    // netty-external-signaling, libdatachannel) are published only here.
    maven {
        name = "OpenCollab"
        url = uri("https://repo.opencollab.dev/main/")
    }
}

val pnxVersion = "3.0.6-SNAPSHOT"

// The PowerNukkitX GitHub release jar, for when repo.powernukkitx.org is unreachable.
// It bundles every library PowerNukkitX uses, so only the tests run against it.
val pnxReleaseJar: String? = providers.gradleProperty("pnxReleaseJar").orNull

// What compile sees of the release jar: PowerNukkitX's own classes and its protocol fork
// (`org.cloudburstmc.protocol`, published only on that repository), and none of the
// libraries bundled beside them.
val pnxThinJar = tasks.register<Jar>("pnxThinJar") {
    onlyIf { pnxReleaseJar != null }
    pnxReleaseJar?.let { release ->
        from(zipTree(release)) {
            include("org/powernukkitx/**", "org/cloudburstmc/protocol/**")
        }
    }
    destinationDirectory.set(layout.buildDirectory.dir("pnx"))
    archiveFileName.set("powernukkitx-thin.jar")
}

dependencies {
    // PowerNukkitX supplies its API, gson, slf4j, snakeyaml and JNA at runtime.
    if (pnxReleaseJar == null) {
        compileOnly("org.powernukkitx:server:$pnxVersion")
    } else {
        compileOnly(files(pnxThinJar))
        // The release jar carries no POM to bring slf4j onto the compile classpath.
        compileOnly("org.slf4j:slf4j-api:2.0.9")
    }

    implementation(project(":common"))

    // The tests mock PowerNukkitX types, so the server has to be on the test
    // runtime classpath as well; compileOnly is not.
    if (pnxReleaseJar == null) {
        testImplementation("org.powernukkitx:server:$pnxVersion")
    } else {
        testCompileOnly(files(pnxThinJar))
        testRuntimeOnly(files(pnxReleaseJar))
    }
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
