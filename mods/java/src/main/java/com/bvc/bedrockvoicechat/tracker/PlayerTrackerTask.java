package com.bvc.bedrockvoicechat.tracker;

import com.bvc.bedrockvoicechat.config.ModConfig;
import com.bvc.bedrockvoicechat.dto.Payload;
import com.bvc.bedrockvoicechat.dto.Player;
import com.bvc.bedrockvoicechat.network.HttpRequestHandler;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.network.ServerPlayerEntity;

import java.net.http.HttpClient;
import java.util.ArrayList;
import java.util.List;

public class PlayerTrackerTask {
    public static void execute(MinecraftServer server, ModConfig config, HttpClient httpClient) {
        List<ServerPlayerEntity> players = server.getPlayerManager().getPlayerList();

        if (players.size() < Math.min(config.getMinimumPlayers(), 2)) {
            return;
        }

        List<Player> playerDataList = new ArrayList<>();
        for (ServerPlayerEntity player : players) {
            playerDataList.add(new Player(player));
        }

        Payload payload = new Payload(playerDataList);
        HttpRequestHandler.sendPlayerData(payload, config, httpClient);
    }
}
