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
}
