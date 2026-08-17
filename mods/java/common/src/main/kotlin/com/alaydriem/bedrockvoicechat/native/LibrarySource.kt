package com.alaydriem.bedrockvoicechat.native

/**
 * Where a library's bytes come from.
 *
 * The fat jar carries them; the skinny jar fetches them. This is the only
 * difference between the two artifacts — everything below this point, including
 * verification, is one code path for both.
 */
sealed interface LibrarySource {

    fun read(): ByteArray

    /** Reads the library the jar already carries. */
    class Bundled(
        private val platform: NativePlatform,
        private val fileName: String
    ) : LibrarySource {
        override fun read(): ByteArray {
            val path = "/native/${platform.id}/$fileName"
            val stream = LibrarySource::class.java.getResourceAsStream(path)
                ?: throw IllegalStateException("Bundled library missing at $path")
            return stream.use { it.readBytes() }
        }
    }

    /** Fetches the library from its pinned release asset. */
    class Remote(
        private val fetcher: LibraryFetcher,
        private val url: String
    ) : LibrarySource {
        override fun read(): ByteArray = fetcher.fetch(url)
    }
}
