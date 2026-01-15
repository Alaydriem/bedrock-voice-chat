package com.alaydriem.bedrockvoicechat.dto;

import com.google.gson.annotations.SerializedName;
import lombok.Getter;

/**
 * Platform-independent player data DTO for the voice chat API.
 */
@Getter
public class PlayerData {
    private final String name;
    private final Coordinates coordinates;
    private final Orientation orientation;
    private final String dimension;

    @SerializedName("world_uuid")
    private final String worldUuid;

    private final boolean deafen;

    /**
     * Constructor for Minecraft players (Fabric/Paper).
     * Does not include worldUuid since Minecraft dimensions are global.
     */
    public PlayerData(String name, double x, double y, double z,
                      float yaw, float pitch, Dimension dimension, boolean deafen) {
        this(name, x, y, z, yaw, pitch, dimension, null, deafen);
    }

    /**
     * Constructor for Hytale players with world UUID for isolation.
     */
    public PlayerData(String name, double x, double y, double z,
                      float yaw, float pitch, Dimension.Hytale dimension,
                      String worldUuid) {
        this(name, x, y, z, yaw, pitch, dimension, worldUuid, false);
    }

    /**
     * Full constructor with all fields.
     */
    public PlayerData(String name, double x, double y, double z,
                      float yaw, float pitch, Dimension dimension,
                      String worldUuid, boolean deafen) {
        this.name = name;
        this.coordinates = new Coordinates(x, y, z);
        this.orientation = new Orientation(yaw, pitch);
        this.dimension = dimension != null ? dimension.toApiString() : null;
        this.worldUuid = worldUuid;
        this.deafen = deafen;
    }
}
