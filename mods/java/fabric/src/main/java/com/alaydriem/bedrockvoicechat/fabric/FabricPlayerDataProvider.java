package com.alaydriem.bedrockvoicechat.fabric;

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider;
import com.alaydriem.bedrockvoicechat.dto.Dimension;
import com.alaydriem.bedrockvoicechat.dto.GameType;
import com.alaydriem.bedrockvoicechat.dto.PlayerData;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.network.ServerPlayerEntity;
import net.minecraft.server.world.ServerWorld;

import java.util.ArrayList;
import java.util.List;

/**
 * Fabric-specific player data provider using Minecraft server API.
 */
public class FabricPlayerDataProvider implements PlayerDataProvider {
    private MinecraftServer server;

    public void setServer(MinecraftServer server) {
        this.server = server;
    }

    @Override
    public List<PlayerData> collectPlayers() {
        if (server == null) {
            return List.of();
        }

        List<ServerPlayerEntity> players = server.getPlayerManager().getPlayerList();
        List<PlayerData> playerDataList = new ArrayList<>();

        for (ServerPlayerEntity player : players) {
            Dimension dimension = getDimension(player.getServerWorld());
            PlayerData data = new PlayerData(
                    player.getName().getString(),
                    player.getX(),
                    player.getY(),
                    player.getZ(),
                    player.getYaw(),
                    player.getPitch(),
                    dimension,
                    player.isSneaking()
            );
            playerDataList.add(data);
        }

        return playerDataList;
    }

    @Override
    public GameType getGameType() {
        return GameType.MINECRAFT;
    }

    private Dimension getDimension(ServerWorld world) {
        String dimensionId = world.getRegistryKey().getValue().toString();

        if (dimensionId.contains("overworld")) {
            return Dimension.Minecraft.OVERWORLD;
        } else if (dimensionId.contains("nether")) {
            return Dimension.Minecraft.NETHER;
        } else if (dimensionId.contains("the_end")) {
            return Dimension.Minecraft.THE_END;
        }

        return new Dimension.Custom(dimensionId);
    }
}
