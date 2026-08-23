package com.alaydriem.bedrockvoicechat.native

/**
 * Typed failures from native library resolution.
 *
 * Each subclass that can occur during the host capability check carries the exact
 * value reported for it, so the reported vocabulary is fixed by the type rather
 * than assembled at the call site.
 */
sealed class NativeLibraryError(
    message: String,
    cause: Throwable? = null
) : Exception(message, cause) {

    class UnsupportedPlatform(os: String, arch: String) : NativeLibraryError(
        "Unsupported platform: $os-$arch. Supported: windows-x64, linux-x64, linux-arm64, darwin-arm64"
    )

    class NotInManifest(library: String, platform: String) : NativeLibraryError(
        "Manifest has no entry for $library on $platform"
    )

    class Fetch(
        val outcome: String,
        message: String,
        cause: Throwable? = null
    ) : NativeLibraryError(message, cause)

    class Write(
        val outcome: String,
        message: String,
        cause: Throwable? = null
    ) : NativeLibraryError(message, cause)

    class DigestMismatch(expected: String, actual: String) : NativeLibraryError(
        "Digest mismatch: expected $expected, got $actual"
    )
}
