package com.bvc.bedrockvoicechat.dto;

import lombok.Getter;
import net.minecraft.server.network.ServerPlayerEntity;

@Getter
public class Player {
    private final String game;
    private final String name;
    private final String dimension;
    private final Coordinates coordinates;
    private final boolean deafen;
    private final Orientation orientation;

    public Player(ServerPlayerEntity player) {
        this.game = "minecraft";
        this.name = player.getGameProfile().getName();
        this.dimension = Dimension.fromWorld(player.getServerWorld()).getValue();
        this.coordinates = new Coordinates(player.getPos());
        this.deafen = player.isSneaking();
        this.orientation = new Orientation(player.getYaw(), player.getPitch());
    }
}
