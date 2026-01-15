package com.alaydriem.bedrockvoicechat.config;

import com.google.gson.annotations.SerializedName;
import lombok.Getter;
import lombok.NoArgsConstructor;
import lombok.Setter;

/**
 * Configuration for the BVC mod, shared across all platforms.
 */
@Getter
@Setter
@NoArgsConstructor
public class ModConfig {
    @SerializedName("bvc-server")
    private String bvcServer;

    @SerializedName("access-token")
    private String accessToken;

    @SerializedName("minimum-players")
    private Integer minimumPlayers = 2;

    /**
     * Check if the configuration is valid (has required fields set).
     */
    public boolean isValid() {
        return bvcServer != null && !bvcServer.isBlank() &&
               accessToken != null && !accessToken.isBlank();
    }
}
