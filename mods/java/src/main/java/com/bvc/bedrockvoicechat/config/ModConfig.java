package com.bvc.bedrockvoicechat.config;

import lombok.Getter;
import lombok.NoArgsConstructor;
import lombok.Setter;

@Getter
@Setter
@NoArgsConstructor
public class ModConfig {
    private String bvcServer;
    private String accessToken;
    private Integer minimumPlayers = 2;

    // Validation method
    public boolean isValid() {
        return bvcServer != null && !bvcServer.isBlank() &&
               accessToken != null && !accessToken.isBlank();
    }
}
