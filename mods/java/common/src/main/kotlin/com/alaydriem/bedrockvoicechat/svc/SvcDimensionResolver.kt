package com.alaydriem.bedrockvoicechat.svc

import uniffi.bvc_relay_sdk.BvcDimensionCodec
import java.util.concurrent.ConcurrentHashMap

/**
 * Finds the level a `custom:<hex>` frame names.
 *
 * A modded dimension crosses the peer wire as a hash of its name, so the bridge cannot
 * read the name back. It hashes the names of its own levels the same way and picks the
 * one that matches. `wireForm` is the native codec, which keeps the hash formula in Rust.
 *
 * Candidates are passed on every call, so a level that loads after the first frame for it
 * is found on the next one. The memo holds one entry per level name.
 */
class SvcDimensionResolver(private val wireForm: (String) -> String) {
    private val forms = ConcurrentHashMap<String, String>()

    fun resolve(dimension: String, candidates: Collection<String>): String? =
        candidates.firstOrNull { name -> forms.getOrPut(name) { wireForm(name) } == dimension }

    companion object {
        // Backed by the relay SDK native, held for the bridge's lifetime.
        fun native(): SvcDimensionResolver {
            val codec = BvcDimensionCodec()
            return SvcDimensionResolver(codec::wireForm)
        }
    }
}
