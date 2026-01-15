package com.alaydriem.bedrockvoicechat.paper;

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider;
import com.alaydriem.bedrockvoicechat.dto.Dimension;
import com.alaydriem.bedrockvoicechat.dto.GameType;
import com.alaydriem.bedrockvoicechat.dto.PlayerData;
import org.bukkit.Bukkit;
import org.bukkit.Location;
import org.bukkit.World;
import org.bukkit.entity.Player;

import java.util.ArrayList;
import java.util.List;

/**
 * Paper-specific player data provider using Bukkit API.
 */
public class PaperPlayerDataProvider implements PlayerDataProvider {

    @Override
    public List<PlayerData> collectPlayers() {
        List<PlayerData> playerDataList = new ArrayList<>();

        for (Player player : Bukkit.getOnlinePlayers()) {
            Location location = player.getLocation();
            Dimension dimension = getDimension(location.getWorld());

            PlayerData data = new PlayerData(
                    player.getName(),
                    location.getX(),
                    location.getY(),
                    location.getZ(),
                    location.getYaw(),
                    location.getPitch(),
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

    private Dimension getDimension(World world) {
        if (world == null) {
            return Dimension.Minecraft.OVERWORLD;
        }

        String worldName = world.getName().toLowerCase();

        // Check for standard world names
        if (worldName.equals("world") || worldName.contains("overworld")) {
            return Dimension.Minecraft.OVERWORLD;
        } else if (worldName.contains("nether")) {
            return Dimension.Minecraft.NETHER;
        } else if (worldName.contains("the_end") || worldName.contains("end")) {
            return Dimension.Minecraft.THE_END;
        }

        // Check environment as fallback
        return switch (world.getEnvironment()) {
            case NORMAL -> Dimension.Minecraft.OVERWORLD;
            case NETHER -> Dimension.Minecraft.NETHER;
            case THE_END -> Dimension.Minecraft.THE_END;
            default -> new Dimension.Custom(worldName);
        };
    }
}
