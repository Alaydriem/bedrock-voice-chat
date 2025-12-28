package com.bvc.bedrockvoicechat.dto;

import lombok.Getter;
import net.minecraft.util.math.Vec3d;

@Getter
public class Coordinates {
    private final double x;
    private final double y;
    private final double z;

    public Coordinates(Vec3d pos) {
        this.x = pos.x;
        this.y = pos.y;
        this.z = pos.z;
    }
}
