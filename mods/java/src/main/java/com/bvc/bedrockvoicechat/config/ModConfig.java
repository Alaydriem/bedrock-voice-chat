package com.bvc.bedrockvoicechat.config;

import lombok.Getter;
import lombok.NoArgsConstructor;
import lombok.Setter;

@Getter
@Setter
@NoArgsConstructor
public class ModConfig {
    private String bvcServer;      // Base URL (e.g., "https://api.example.com")
    private String accessToken;     // Authentication token

    // Validation method
    public boolean isValid() {
        return bvcServer != null && !bvcServer.isBlank() &&
               accessToken != null && !accessToken.isBlank();
    }
}
