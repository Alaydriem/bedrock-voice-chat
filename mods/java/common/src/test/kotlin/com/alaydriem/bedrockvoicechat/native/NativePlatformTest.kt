package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class NativePlatformTest {

    @Test
    fun `detects each supported platform from os and arch strings`() {
        assertEquals(NativePlatform.WINDOWS_X64, NativePlatform.detect("Windows 11", "amd64"))
        assertEquals(NativePlatform.LINUX_X64, NativePlatform.detect("Linux", "x86_64"))
        assertEquals(NativePlatform.LINUX_ARM64, NativePlatform.detect("Linux", "aarch64"))
        assertEquals(NativePlatform.DARWIN_ARM64, NativePlatform.detect("Mac OS X", "aarch64"))
    }

    // Windows arm64 and macOS x64 are deliberately unsupported. They must fail by
    // name here rather than resolve to a file that does not exist and surface
    // later as UnsatisfiedLinkError.
    @Test
    fun `refuses an unsupported os and arch combination by name`() {
        val error = assertThrows(NativeLibraryError.UnsupportedPlatform::class.java) {
            NativePlatform.detect("Windows 11", "aarch64")
        }
        assertTrue(error.message!!.contains("windows"))
        assertTrue(error.message!!.contains("arm64"))
    }

    @Test
    fun `names the library file per platform convention`() {
        assertEquals("bvc_server_lib.dll", NativePlatform.WINDOWS_X64.fileNameFor("bvc_server_lib"))
        assertEquals("libbvc_server_lib.so", NativePlatform.LINUX_X64.fileNameFor("bvc_server_lib"))
        assertEquals("libbvc_relay_sdk.dylib", NativePlatform.DARWIN_ARM64.fileNameFor("bvc_relay_sdk"))
    }
}
