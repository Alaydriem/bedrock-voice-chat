package com.alaydriem.bedrockvoicechat.fabric.audio

import net.fabricmc.fabric.api.`object`.builder.v1.block.entity.FabricBlockEntityTypeBuilder
import net.minecraft.block.AbstractBlock
import net.minecraft.block.Block
import net.minecraft.block.entity.BlockEntityType
import net.minecraft.component.ComponentType
import net.minecraft.item.BlockItem
import net.minecraft.item.Item
import net.minecraft.registry.Registries
import net.minecraft.registry.Registry
import net.minecraft.util.Identifier
import org.slf4j.LoggerFactory

/**
 * Registers BVC audio blocks, items, and block entity types with the Fabric registry.
 */
object FabricAudioRegistry {
    private val logger = LoggerFactory.getLogger("BVC Audio Registry")

    // Component types for storing audio data on items
    lateinit var BVC_AUDIO_ID: ComponentType<String>
        private set
    lateinit var BVC_EVENT_ID: ComponentType<String>
        private set

    // Block and item instances
    lateinit var AUDIO_PLAYER_BLOCK: AudioPlayerBlock
        private set
    lateinit var AUDIO_PLAYER_BLOCK_ITEM: BlockItem
        private set
    lateinit var AUDIO_DISC_ITEM: AudioDiscItem
        private set
    lateinit var AUDIO_PLAYER_BLOCK_ENTITY_TYPE: BlockEntityType<AudioPlayerBlockEntity>
        private set

    fun register() {
        logger.info("Registering BVC audio blocks and items")

        // Register data component types
        BVC_AUDIO_ID = Registry.register(
            Registries.DATA_COMPONENT_TYPE,
            Identifier.of("bvc", "audio_id"),
            ComponentType.builder<String>()
                .codec(com.mojang.serialization.Codec.STRING)
                .build()
        )

        BVC_EVENT_ID = Registry.register(
            Registries.DATA_COMPONENT_TYPE,
            Identifier.of("bvc", "event_id"),
            ComponentType.builder<String>()
                .codec(com.mojang.serialization.Codec.STRING)
                .build()
        )

        // Register block
        AUDIO_PLAYER_BLOCK = AudioPlayerBlock(
            AbstractBlock.Settings.create()
                .strength(2.0f)
                .requiresTool()
        )
        Registry.register(
            Registries.BLOCK,
            Identifier.of("bvc", "audio_player"),
            AUDIO_PLAYER_BLOCK
        )

        // Register block item
        AUDIO_PLAYER_BLOCK_ITEM = BlockItem(
            AUDIO_PLAYER_BLOCK,
            Item.Settings()
        )
        Registry.register(
            Registries.ITEM,
            Identifier.of("bvc", "audio_player"),
            AUDIO_PLAYER_BLOCK_ITEM
        )

        // Register disc item
        AUDIO_DISC_ITEM = AudioDiscItem(Item.Settings().maxCount(1))
        Registry.register(
            Registries.ITEM,
            Identifier.of("bvc", "audio_disc"),
            AUDIO_DISC_ITEM
        )

        // Register block entity type
        AUDIO_PLAYER_BLOCK_ENTITY_TYPE = Registry.register(
            Registries.BLOCK_ENTITY_TYPE,
            Identifier.of("bvc", "audio_player"),
            FabricBlockEntityTypeBuilder.create(
                { pos, state -> AudioPlayerBlockEntity(pos, state) },
                AUDIO_PLAYER_BLOCK
            ).build()
        )

        logger.info("BVC audio blocks and items registered")
    }
}
