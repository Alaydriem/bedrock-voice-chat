package com.alaydriem.bedrockvoicechat.config

import com.google.gson.annotations.SerializedName

/**
 * Mirrors the server-side BedrockDnsConfig. Disabled by default; a Java server
 * fronts Bedrock crossplay with Geyser, so BVC's DNS override is opt-in.
 */
class EmbeddedBedrockDnsConfig {
    @SerializedName("enabled")
    var enabled: Boolean = false

    @SerializedName("port")
    var port: Int = 53

    @SerializedName("upstream")
    var upstream: List<String> = listOf("1.1.1.1", "1.0.0.1")

    @SerializedName(value = "override-host", alternate = ["overrideHost"])
    var overrideHost: String = "geo.hivebedrock.network"

    @SerializedName(value = "rate-limit-per-sec", alternate = ["rateLimitPerSec"])
    var rateLimitPerSec: Int = 100
}
