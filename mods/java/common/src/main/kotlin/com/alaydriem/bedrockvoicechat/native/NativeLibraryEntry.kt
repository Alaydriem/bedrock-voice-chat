package com.alaydriem.bedrockvoicechat.native

/**
 * One library, on one platform: the release asset that carries it and the digest
 * that must match before it is loaded.
 */
data class NativeLibraryEntry(
    val asset: String,
    val sha256: String
)
