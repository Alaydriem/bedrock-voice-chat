package com.bvc.bedrockvoicechat.tracker;

import com.bvc.bedrockvoicechat.config.ModConfig;
import com.bvc.bedrockvoicechat.dto.Player;
import com.bvc.bedrockvoicechat.network.HttpRequestHandler;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.network.ServerPlayerEntity;

import java.net.http.HttpClient;
import java.util.ArrayList;
import java.util.List;

public class PlayerTrackerTask {
    public static void execute(MinecraftServer server, ModConfig config, HttpClient httpClient) {
        // Get all online players
        List<ServerPlayerEntity> players = server.getPlayerManager().getPlayerList();

        // Only send if 2+ players online (matches BDS)
        if (players.size() <= 1) {
            return;
        }

        // Collect player data using Player DTO
        List<Player> playerDataList = new ArrayList<>();
        for (ServerPlayerEntity player : players) {
            playerDataList.add(new Player(player));
        }

        // Send data asynchronously
        HttpRequestHandler.sendPlayerData(playerDataList, config, httpClient);
    }
}
