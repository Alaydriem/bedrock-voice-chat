package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class BvcNativeTest {

    // The loader must ask the provider for a path rather than extracting a
    // resource itself. Without a configured provider it must say so, rather than
    // silently falling back to a system-path lookup that could load any library
    // of that name already present on the host.
    @Test
    fun `loading without a configured provider fails by name`() {
        BvcNative.configure(null)

        val error = assertThrows(IllegalStateException::class.java) {
            BvcNative.load()
        }

        assertTrue(
            error.message!!.contains("provider"),
            "the message must name what is missing, was: ${error.message}"
        )
    }
}
