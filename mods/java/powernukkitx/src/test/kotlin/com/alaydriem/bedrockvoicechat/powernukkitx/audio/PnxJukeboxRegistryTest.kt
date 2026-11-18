package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import org.mockito.kotlin.mock
import org.powernukkitx.level.Level

class PnxJukeboxRegistryTest {

    @Test
    fun unloading_a_level_drops_only_its_jukeboxes() {
        val unloaded = mock<Level>()
        val kept = mock<Level>()
        val registry = PnxJukeboxRegistry()
        registry.add(PnxJukeboxLocation(unloaded, 1, 64, 1))
        registry.add(PnxJukeboxLocation(unloaded, 20, 64, 20))
        val survivor = PnxJukeboxLocation(kept, 1, 64, 1)
        registry.add(survivor)

        registry.removeLevel(unloaded)

        assertEquals(listOf(survivor), registry.all())
    }
}
