package com.alaydriem.bedrockvoicechat.dto;

/**
 * Sealed interface for game-specific dimensions.
 * Each game platform has its own dimension variants.
 */
public sealed interface Dimension permits
        Dimension.Minecraft,
        Dimension.Hytale,
        Dimension.Custom {

    /**
     * Minecraft dimensions.
     */
    enum Minecraft implements Dimension {
        OVERWORLD("overworld"),
        NETHER("nether"),
        THE_END("the_end");

        private final String value;

        Minecraft(String value) {
            this.value = value;
        }

        @Override
        public String toApiString() {
            return value;
        }
    }

    /**
     * Hytale dimensions.
     */
    enum Hytale implements Dimension {
        ORBIS("orbis");

        private final String value;

        Hytale(String value) {
            this.value = value;
        }

        @Override
        public String toApiString() {
            return value;
        }
    }

    /**
     * Custom/unknown dimensions for extensibility.
     */
    record Custom(String name) implements Dimension {
        @Override
        public String toApiString() {
            return name;
        }
    }

    /**
     * Parse a dimension from a platform-specific string.
     *
     * @param game The game type
     * @param raw  The raw dimension string from the platform
     * @return The parsed Dimension, or null if raw is null
     */
    static Dimension fromString(GameType game, String raw) {
        if (raw == null) {
            return null;
        }

        return switch (game) {
            case MINECRAFT -> switch (raw.toLowerCase()) {
                case "minecraft:overworld", "overworld", "world" -> Minecraft.OVERWORLD;
                case "minecraft:the_nether", "the_nether", "nether", "world_nether" -> Minecraft.NETHER;
                case "minecraft:the_end", "the_end", "world_the_end" -> Minecraft.THE_END;
                default -> new Custom(raw);
            };
            case HYTALE -> switch (raw.toLowerCase()) {
                case "orbis" -> Hytale.ORBIS;
                default -> new Custom(raw);
            };
        };
    }

    /**
     * Serialize this dimension to the API string format.
     *
     * @return The API string representation
     */
    String toApiString();
}
