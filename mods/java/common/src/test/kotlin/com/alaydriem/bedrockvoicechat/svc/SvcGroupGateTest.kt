package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.Group
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.util.UUID

class SvcGroupGateTest {

    private class FakeGroup(
        private val type: Group.Type,
        private val password: Boolean = false,
        private val hidden: Boolean = false
    ) : Group {
        override fun getName(): String = "group"
        override fun hasPassword(): Boolean = password
        override fun getId(): UUID = UUID.randomUUID()
        override fun isPersistent(): Boolean = false
        override fun isHidden(): Boolean = hidden
        override fun getType(): Group.Type = type
    }

    @Test
    fun `a speaker outside any group is bridged`() {
        assertTrue(SvcGroupGate.bridges(null))
    }

    // Nearby players hear an OPEN group in SVC, so they may hear it from Bedrock too.
    @Test
    fun `an open group speaker is bridged`() {
        assertTrue(SvcGroupGate.bridges(FakeGroup(Group.Type.OPEN)))
    }

    @Test
    fun `a normal group speaker is not bridged`() {
        assertFalse(SvcGroupGate.bridges(FakeGroup(Group.Type.NORMAL)))
    }

    @Test
    fun `an isolated group speaker is not bridged`() {
        assertFalse(SvcGroupGate.bridges(FakeGroup(Group.Type.ISOLATED)))
    }

    // A password or a hidden flag limits who can join, not who can hear.
    @Test
    fun `a password protected or hidden open group is still bridged`() {
        assertTrue(SvcGroupGate.bridges(FakeGroup(Group.Type.OPEN, password = true, hidden = true)))
    }
}
