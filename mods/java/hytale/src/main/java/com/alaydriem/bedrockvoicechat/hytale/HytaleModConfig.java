package com.alaydriem.bedrockvoicechat.hytale;

import com.hypixel.hytale.codec.Codec;
import com.hypixel.hytale.codec.KeyedCodec;
import com.hypixel.hytale.codec.builder.BuilderCodec;

/**
 * Hytale-native config class using BuilderCodec.
 * Mirrors ModConfig fields but uses Hytale's serialization.
 */
public class HytaleModConfig {
    public static final BuilderCodec<HytaleModConfig> CODEC =
        BuilderCodec.builder(HytaleModConfig.class, HytaleModConfig::new)
            .append(
                new KeyedCodec<>("bvc-server", Codec.STRING),
                (cfg, val) -> cfg.bvcServer = val,
                cfg -> cfg.bvcServer
            ).add()
            .append(
                new KeyedCodec<>("access-token", Codec.STRING),
                (cfg, val) -> cfg.accessToken = val,
                cfg -> cfg.accessToken
            ).add()
            .append(
                new KeyedCodec<>("minimum-players", Codec.INTEGER),
                (cfg, val) -> cfg.minimumPlayers = val,
                cfg -> cfg.minimumPlayers
            ).add()
            .build();

    private String bvcServer = "";
    private String accessToken = "";
    private int minimumPlayers = 2;

    public HytaleModConfig() {}

    public String getBvcServer() {
        return bvcServer;
    }

    public String getAccessToken() {
        return accessToken;
    }

    public int getMinimumPlayers() {
        return minimumPlayers;
    }

    public boolean isValid() {
        return bvcServer != null && !bvcServer.isBlank()
            && accessToken != null && !accessToken.isBlank();
    }
}
