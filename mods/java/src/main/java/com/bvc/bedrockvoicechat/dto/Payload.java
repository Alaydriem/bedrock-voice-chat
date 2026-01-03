package com.alaydriem.bedrockvoicechat.dto;

import lombok.Getter;

import java.util.List;

@Getter
public class Payload {
    private final String game;
    private final List<Player> players;

    public Payload(List<Player> players) {
        this.game = "minecraft";
        this.players = players;
    }
}
