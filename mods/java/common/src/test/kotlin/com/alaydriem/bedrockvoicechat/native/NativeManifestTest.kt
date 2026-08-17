package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class NativeManifestTest {

    private val json = """
        {
          "release": "mods-v1.2.3",
          "base_url": "https://github.com/alaydriem/bedrock-voice-chat/releases/download/mods-v1.2.3",
          "libraries": {
            "bvc_server_lib": {
              "linux-x64": { "asset": "libbvc_server_lib-linux-x64.so", "sha256": "aaaa" },
              "windows-x64": { "asset": "bvc_server_lib-windows-x64.dll", "sha256": "bbbb" }
            },
            "bvc_relay_sdk": {
              "linux-x64": { "asset": "libbvc_relay_sdk-linux-x64.so", "sha256": "cccc" }
            }
          }
        }
    """.trimIndent()

    @Test
    fun `resolves an entry by library and platform`() {
        val manifest = NativeManifest.parse(json)
        val entry = manifest.entry("bvc_server_lib", NativePlatform.LINUX_X64)

        assertEquals("libbvc_server_lib-linux-x64.so", entry.asset)
        assertEquals("aaaa", entry.sha256)
    }

    @Test
    fun `builds an asset url under the pinned release`() {
        val manifest = NativeManifest.parse(json)
        val entry = manifest.entry("bvc_relay_sdk", NativePlatform.LINUX_X64)

        assertEquals(
            "https://github.com/alaydriem/bedrock-voice-chat/releases/download/mods-v1.2.3/libbvc_relay_sdk-linux-x64.so",
            manifest.assetUrl(entry)
        )
    }

    // A platform the release did not build for must fail here, where the message
    // names the library and platform, rather than at load time.
    @Test
    fun `a missing platform entry fails naming the library and platform`() {
        val manifest = NativeManifest.parse(json)

        val error = assertThrows(NativeLibraryError.NotInManifest::class.java) {
            manifest.entry("bvc_relay_sdk", NativePlatform.DARWIN_ARM64)
        }
        assertTrue(error.message!!.contains("bvc_relay_sdk"))
        assertTrue(error.message!!.contains("darwin-arm64"))
    }

    @Test
    fun `the manifest asset sits beside the libraries`() {
        val manifest = NativeManifest.parse(json)

        assertEquals(
            "https://github.com/alaydriem/bedrock-voice-chat/releases/download/mods-v1.2.3/native-manifest.json",
            manifest.manifestUrl()
        )
    }
}
