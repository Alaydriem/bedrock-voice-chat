package com.alaydriem.bedrockvoicechat.dto;

import lombok.Getter;

@Getter
public class Orientation {
    // Yaw (horizontal rotation)
    private final float x;

    // Pitch (vertical rotation)
    private final float y;

    public Orientation(float yaw, float pitch) {
        this.x = yaw;
        this.y = pitch;
    }
}
