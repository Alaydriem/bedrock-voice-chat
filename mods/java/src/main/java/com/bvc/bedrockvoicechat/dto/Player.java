package com.alaydriem.bedrockvoicechat.dto;

import lombok.Getter;
import net.minecraft.server.network.ServerPlayerEntity;

@Getter
public class Player {
    private final String name;
    private final String dimension;
    private final Coordinates coordinates;
    private final Orientation orientation;
    private final boolean deafen;

    public Player(ServerPlayerEntity player) {
        this.name = player.getName().getString();
        this.dimension = Dimension.fromWorld(player.getServerWorld()).getValue();
        this.coordinates = new Coordinates(player.getX(), player.getY(), player.getZ());
        this.deafen = player.isSneaking();
        this.orientation = new Orientation(player.getYaw(), player.getPitch());
    }
}
