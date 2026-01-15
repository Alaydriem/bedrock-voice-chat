package com.alaydriem.bedrockvoicechat.hytale;

import com.alaydriem.bedrockvoicechat.api.PlayerDataProvider;
import com.alaydriem.bedrockvoicechat.dto.Dimension;
import com.alaydriem.bedrockvoicechat.dto.GameType;
import com.alaydriem.bedrockvoicechat.dto.PlayerData;

import com.hypixel.hytale.server.core.universe.PlayerRef;
import com.hypixel.hytale.math.vector.Vector3d;
import com.hypixel.hytale.math.vector.Vector3f;

import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Hytale-specific player data provider using PlayerRef API.
 */
public class HytalePlayerDataProvider implements PlayerDataProvider {
    private final Set<PlayerRef> onlinePlayers = ConcurrentHashMap.newKeySet();

    public void addPlayer(PlayerRef player) {
        onlinePlayers.add(player);
    }

    public void removePlayer(PlayerRef player) {
        onlinePlayers.remove(player);
    }

    @Override
    public List<PlayerData> collectPlayers() {
        return onlinePlayers.stream()
                .filter(PlayerRef::isValid)
                .map(this::toPlayerData)
                .toList();
    }

    @Override
    public GameType getGameType() {
        return GameType.HYTALE;
    }

    private PlayerData toPlayerData(PlayerRef ref) {
        // Get position from transform
        Vector3d pos = ref.getTransform().getPosition();

        // Get head rotation for orientation (yaw, pitch)
        Vector3f rot = ref.getHeadRotation();

        // Get world UUID for world isolation
        String worldUuid = ref.getWorldUuid().toString();

        return new PlayerData(
                ref.getUsername(),
                pos.x, pos.y, pos.z,
                rot.x, rot.y,  // yaw, pitch from head rotation
                Dimension.Hytale.ORBIS,
                worldUuid
        );
    }
}
