package com.alaydriem.bedrockvoicechat.dto;

import lombok.Getter;

/**
 * Player orientation (yaw and pitch).
 */
@Getter
public class Orientation {
    /**
     * Yaw (horizontal rotation) - stored as 'x' for API compatibility.
     */
    private final float x;

    /**
     * Pitch (vertical rotation) - stored as 'y' for API compatibility.
     */
    private final float y;

    public Orientation(float yaw, float pitch) {
        this.x = yaw;
        this.y = pitch;
    }
}
