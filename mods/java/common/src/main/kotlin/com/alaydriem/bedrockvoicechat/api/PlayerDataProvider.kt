package com.alaydriem.bedrockvoicechat.api

import com.alaydriem.bedrockvoicechat.dto.GameType
import com.alaydriem.bedrockvoicechat.dto.PlayerData

/**
 * Interface for platform-specific player data collection.
 */
interface PlayerDataProvider {
    /**
     * Collect current player data from all online players.
     *
     * @return List of player data DTOs ready for API submission
     */
    fun collectPlayers(): List<PlayerData>

    /**
     * Get the game type for this platform.
     */
    fun getGameType(): GameType
}
