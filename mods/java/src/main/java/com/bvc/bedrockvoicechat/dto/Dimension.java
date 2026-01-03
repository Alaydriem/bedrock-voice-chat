package com.alaydriem.bedrockvoicechat.dto;

import net.minecraft.server.world.ServerWorld;

public enum Dimension {
    OVERWORLD("overworld"),
    THE_NETHER("nether"),
    THE_END("the_end");

    private final String value;

    Dimension(String value) {
        this.value = value;
    }

    public String getValue() {
        return value;
    }

    public static Dimension fromWorld(ServerWorld world) {
        String dimensionId = world.getRegistryKey().getValue().toString();

        if (dimensionId.contains("overworld")) {
            return OVERWORLD;
        } else if (dimensionId.contains("nether")) {
            return THE_NETHER;
        } else if (dimensionId.contains("the_end")) {
            return THE_END;
        }

        return OVERWORLD; // Default fallback
    }
}
