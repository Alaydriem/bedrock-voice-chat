package com.alaydriem.bedrockvoicechat.native

import com.sun.jna.Pointer
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.lang.reflect.Proxy

class BvcNativeHostTest {

    private fun library(setHost: (String) -> Int): BvcNative.BvcLibrary =
        Proxy.newProxyInstance(
            BvcNative.BvcLibrary::class.java.classLoader,
            arrayOf(BvcNative.BvcLibrary::class.java)
        ) { _, method, args ->
            when (method.name) {
                "bvc_server_set_host" -> setHost(args[1] as String)
                else -> throw UnsupportedOperationException(method.name)
            }
        } as BvcNative.BvcLibrary

    @Test
    fun `a missing set_host export does not throw`() {
        val lib = library { throw UnsatisfiedLinkError("bvc_server_set_host") }

        assertFalse(BvcNative.setServerHost(lib, Pointer(1), "paper"))
    }

    @Test
    fun `the host kind reaches the export and a zero return is success`() {
        var seen: String? = null
        val lib = library { seen = it; 0 }

        assertTrue(BvcNative.setServerHost(lib, Pointer(1), "powernukkitx"))
        assertEquals("powernukkitx", seen)
    }

    @Test
    fun `a refusal from the server is reported as false`() {
        val lib = library { -1 }

        assertFalse(BvcNative.setServerHost(lib, Pointer(1), "paper"))
    }
}
