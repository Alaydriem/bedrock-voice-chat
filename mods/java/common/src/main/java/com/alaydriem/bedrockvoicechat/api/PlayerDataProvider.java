package com.alaydriem.bedrockvoicechat.api;

import com.alaydriem.bedrockvoicechat.dto.GameType;
import com.alaydriem.bedrockvoicechat.dto.PlayerData;

import java.util.List;

/**
 * Interface for platform-specific player data collection.
 */
public interface PlayerDataProvider {
    /**
     * Collect current player data from all online players.
     *
     * @return List of player data DTOs ready for API submission
     */
    List<PlayerData> collectPlayers();

    /**
     * Get the game type for this platform.
     *
     * @return The game type
     */
    GameType getGameType();
}
