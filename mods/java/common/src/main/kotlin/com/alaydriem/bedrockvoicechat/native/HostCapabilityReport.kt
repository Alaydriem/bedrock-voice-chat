package com.alaydriem.bedrockvoicechat.native

import com.google.gson.annotations.SerializedName

/**
 * Whether this host could run the skinny jar.
 *
 * Seven fields, and no more. No hostname, no address, no file path, no player or
 * world data — the question is whether a host of this shape can fetch and write,
 * and nothing else is needed to answer it. The loader and mode name the shape of
 * the host, and the server accepts them only together.
 */
data class HostCapabilityReport(
    val variant: String,
    val platform: String,
    @SerializedName("mod_version")
    val modVersion: String,
    val fetch: String,
    val write: String,
    val loader: String,
    val mode: String
)
