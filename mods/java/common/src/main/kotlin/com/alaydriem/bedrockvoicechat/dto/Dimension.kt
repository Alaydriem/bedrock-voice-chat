package com.alaydriem.bedrockvoicechat.dto

/**
 * Sealed interface for game-specific dimensions.
 * Each game platform has its own dimension variants.
 */
sealed interface Dimension {
    /**
     * Serialize this dimension to the API string format.
     */
    fun toApiString(): String

    /**
     * Minecraft dimensions.
     */
    enum class Minecraft(private val value: String) : Dimension {
        OVERWORLD("overworld"),
        NETHER("nether"),
        THE_END("the_end"),
        DEATH("death");

        override fun toApiString(): String = value
    }

    /**
     * Hytale dimensions.
     */
    enum class Hytale(private val value: String) : Dimension {
        ORBIS("orbis"),
        DEATH("death");

        override fun toApiString(): String = value
    }

    /**
     * Custom/unknown dimensions for extensibility.
     */
    data class Custom(val name: String) : Dimension {
        override fun toApiString(): String = name
    }

    companion object {
        /**
         * Parse a dimension from a platform-specific string.
         *
         * @param game The game type
         * @param raw The raw dimension string from the platform
         * @return The parsed Dimension, or null if raw is null
         */
        fun fromString(game: GameType, raw: String?): Dimension? {
            if (raw == null) return null

            return when (game) {
                GameType.MINECRAFT -> when (raw.lowercase()) {
                    "minecraft:overworld", "overworld", "world" -> Minecraft.OVERWORLD
                    "minecraft:the_nether", "the_nether", "nether", "world_nether" -> Minecraft.NETHER
                    "minecraft:the_end", "the_end", "world_the_end" -> Minecraft.THE_END
                    "death" -> Minecraft.DEATH
                    else -> Custom(raw)
                }
                GameType.HYTALE -> when (raw.lowercase()) {
                    "orbis" -> Hytale.ORBIS
                    "death" -> Hytale.DEATH
                    else -> Custom(raw)
                }
            }
        }
    }
}
