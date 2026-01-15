package com.alaydriem.bedrockvoicechat.dto;

import lombok.Getter;

import java.util.List;

/**
 * Payload sent to the BVC server with player position data.
 */
@Getter
public class Payload {
    private final String game;
    private final List<PlayerData> players;

    public Payload(GameType gameType, List<PlayerData> players) {
        this.game = gameType.getValue();
        this.players = players;
    }
}
