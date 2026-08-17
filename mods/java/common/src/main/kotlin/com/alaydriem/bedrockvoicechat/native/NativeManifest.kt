package com.alaydriem.bedrockvoicechat.native

import com.google.gson.Gson
import com.google.gson.annotations.SerializedName

/**
 * The pinned set of native libraries a jar was built against.
 *
 * `release` names an exact tag. Nothing here resolves a moving target: a jar must
 * fetch the library it was built against, and a later release must not be able to
 * change the native under an installed jar.
 */
class NativeManifest(
    val release: String,
    @SerializedName("base_url")
    val baseUrl: String,
    val libraries: Map<String, Map<String, NativeLibraryEntry>>
) {
    fun entry(library: String, platform: NativePlatform): NativeLibraryEntry =
        libraries[library]?.get(platform.id)
            ?: throw NativeLibraryError.NotInManifest(library, platform.id)

    fun assetUrl(entry: NativeLibraryEntry): String = "$baseUrl/${entry.asset}"

    fun manifestUrl(): String = "$baseUrl/$MANIFEST_NAME"

    companion object {
        const val MANIFEST_NAME: String = "native-manifest.json"

        private const val RESOURCE_PATH: String = "/$MANIFEST_NAME"

        fun parse(json: String): NativeManifest = Gson().fromJson(json, NativeManifest::class.java)

        /**
         * Reads the manifest the jar was built with. This is the trust root: the
         * digests it carries are what make fetched bytes as trustworthy as
         * bundled bytes.
         */
        fun fromResources(): NativeManifest {
            val stream = NativeManifest::class.java.getResourceAsStream(RESOURCE_PATH)
                ?: throw IllegalStateException("$MANIFEST_NAME is missing from the jar")
            return parse(stream.use { it.readBytes().decodeToString() })
        }
    }
}
