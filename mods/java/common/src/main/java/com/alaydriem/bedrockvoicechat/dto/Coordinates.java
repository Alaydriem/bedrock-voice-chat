package com.alaydriem.bedrockvoicechat.dto;

import lombok.Getter;

/**
 * 3D coordinates for player position.
 */
@Getter
public class Coordinates {
    private final double x;
    private final double y;
    private final double z;

    public Coordinates(double x, double y, double z) {
        this.x = x;
        this.y = y;
        this.z = z;
    }
}
