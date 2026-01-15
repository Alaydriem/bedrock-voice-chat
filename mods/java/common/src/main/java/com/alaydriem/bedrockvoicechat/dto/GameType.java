package com.alaydriem.bedrockvoicechat.dto;

import com.google.gson.annotations.SerializedName;

/**
 * Supported game platforms for voice chat integration.
 */
public enum GameType {
    @SerializedName("minecraft")
    MINECRAFT("minecraft"),

    @SerializedName("hytale")
    HYTALE("hytale");

    private final String value;

    GameType(String value) {
        this.value = value;
    }

    public String getValue() {
        return value;
    }
}
