package com.alaydriem.bedrockvoicechat.native

/**
 * The four supported native platforms, their identifiers and their library naming.
 *
 * `detect` takes the strings rather than reading system properties, so the whole
 * table is testable without a machine of each shape.
 */
enum class NativePlatform(
    val id: String,
    private val libPrefix: String,
    private val libExtension: String
) {
    WINDOWS_X64("windows-x64", "", "dll"),
    LINUX_X64("linux-x64", "lib", "so"),
    LINUX_ARM64("linux-arm64", "lib", "so"),
    DARWIN_ARM64("darwin-arm64", "lib", "dylib");

    fun fileNameFor(library: String): String = "$libPrefix$library.$libExtension"

    companion object {
        fun current(): NativePlatform = detect(
            System.getProperty("os.name"),
            System.getProperty("os.arch")
        )

        fun detect(osName: String, osArch: String): NativePlatform {
            val os = normaliseOs(osName)
            val arch = normaliseArch(osArch)

            return when {
                os == "windows" && arch == "x64" -> WINDOWS_X64
                os == "linux" && arch == "x64" -> LINUX_X64
                os == "linux" && arch == "arm64" -> LINUX_ARM64
                os == "darwin" && arch == "arm64" -> DARWIN_ARM64
                else -> throw NativeLibraryError.UnsupportedPlatform(os, arch)
            }
        }

        private fun normaliseOs(osName: String): String {
            val os = osName.lowercase()
            return when {
                os.contains("win") -> "windows"
                os.contains("mac") || os.contains("darwin") -> "darwin"
                os.contains("linux") -> "linux"
                else -> os
            }
        }

        private fun normaliseArch(osArch: String): String {
            val arch = osArch.lowercase()
            return when {
                arch.contains("amd64") || arch.contains("x86_64") -> "x64"
                arch.contains("aarch64") || arch.contains("arm64") -> "arm64"
                else -> arch
            }
        }
    }
}
