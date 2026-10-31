package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

class SvcDimensionResolverTest {
    private val calls = mutableListOf<String>()

    // Stands in for the native codec: any deterministic name-to-form map will do.
    private val resolver = SvcDimensionResolver { name ->
        calls += name
        "custom:$name"
    }

    @Test
    fun `resolves the candidate whose wire form matches`() {
        assertEquals("aether", resolver.resolve("custom:aether", listOf("overworld_copy", "aether")))
    }

    @Test
    fun `returns null when no candidate matches`() {
        assertNull(resolver.resolve("custom:aether", listOf("lobby")))
    }

    // A modded dimension can load after the first frame for it arrives. A cached miss
    // would leave its speakers silent until restart.
    @Test
    fun `finds a level that loaded after the first miss`() {
        assertNull(resolver.resolve("custom:b", listOf("a")))
        assertEquals("b", resolver.resolve("custom:b", listOf("a", "b")))
    }

    // The codec crosses FFI; hashing every level on every frame would be paid per packet.
    @Test
    fun `hashes each candidate once`() {
        repeat(3) { resolver.resolve("custom:z", listOf("a", "b")) }

        assertEquals(listOf("a", "b"), calls)
    }
}
