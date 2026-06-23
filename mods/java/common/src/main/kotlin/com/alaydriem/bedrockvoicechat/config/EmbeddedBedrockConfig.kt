package com.alaydriem.bedrockvoicechat.config

import com.google.gson.annotations.SerializedName

/**
 * Mirrors the server-side BedrockConfig. Enabled by default, but the transfer
 * relay binds 19139 (not the server-side 19132 default) so it coexists with
 * Geyser, which owns 19132 on a Java server.
 */
class EmbeddedBedrockConfig {
    @SerializedName("enabled")
    var enabled: Boolean = false

    @SerializedName(value = "transfer-port", alternate = ["transferPort"])
    var transferPort: Int = 19139

    @SerializedName(value = "transfer-target-port", alternate = ["transferTargetPort"])
    var transferTargetPort: Int = 19137

    @SerializedName(value = "transfer-cache-ttl-secs", alternate = ["transferCacheTtlSecs"])
    var transferCacheTtlSecs: Long = 900

    @SerializedName(
        value = "proxy-event-freshness-threshold-secs",
        alternate = ["proxyEventFreshnessThresholdSecs"]
    )
    var proxyEventFreshnessThresholdSecs: Int = 30

    @SerializedName("dns")
    var dns: EmbeddedBedrockDnsConfig = EmbeddedBedrockDnsConfig()
}
