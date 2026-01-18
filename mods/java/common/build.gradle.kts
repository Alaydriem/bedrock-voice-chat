plugins {
    `java-library`
}

val archivesBaseName: String by project

base {
    archivesName.set("$archivesBaseName-common")
}

dependencies {
    // Gson for JSON serialization
    api("com.google.code.gson:gson:2.10.1")

    // JNA for native library loading (FFI with Rust BVC server)
    api("net.java.dev.jna:jna:5.14.0")

    // SLF4J for logging (provided by platform implementations)
    compileOnly("org.slf4j:slf4j-api:2.0.9")
}
