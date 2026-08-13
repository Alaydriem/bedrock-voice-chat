import { get } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke } from "../../tauri";
import { BedrockCapabilityManager } from "../../../js/app/managers/bedrock/BedrockCapabilityManager";

function configWith(servers: unknown[]) {
    return {
        config: {
            status: "Ok",
            client_id: "cid",
            protocol_version: "3.0.0",
            quic_port: 0,
            quic_ports: [1],
            bedrock: {
                enabled: true,
                transfer_port: 19132,
                servers,
            },
        },
        client_version: "3.0.0",
        compatible: true,
        client_too_old: false,
    };
}

describe("Bedrock capability manager", () => {
    beforeEach(() => {
        mockInvoke({});
    });

    it("carries the declared addon mode onto each advertised entry", async () => {
        mockInvoke({
            api_get_config: () =>
                configWith([
                    {
                        name: "Truly Bedrock SMP",
                        host: "tbs7.nodecraft.gg",
                        port: 19132,
                        protocol_version: null,
                        addon_mode: "net",
                    },
                ]),
        });

        const capability = new BedrockCapabilityManager();
        await capability.refresh();

        const entries = get(capability.serverProvidedServers);
        expect(entries).toHaveLength(1);
        expect(entries[0].host).toBe("tbs7.nodecraft.gg");
        expect(entries[0].addonMode).toBe("net");
        expect(entries[0].source).toBe("server");
        capability.destroy();
    });

    // A null protocol version means Auto and must not be copied onto the entry,
    // where a present-but-null field would read as an explicit pin.
    it("omits the protocol version when the server sends null", async () => {
        mockInvoke({
            api_get_config: () =>
                configWith([
                    {
                        name: "Auto",
                        host: "play.example.com",
                        port: 19132,
                        protocol_version: null,
                        addon_mode: "no_net",
                    },
                ]),
        });

        const capability = new BedrockCapabilityManager();
        await capability.refresh();

        const entries = get(capability.serverProvidedServers);
        expect("protocolVersion" in entries[0]).toBe(false);
        expect(entries[0].addonMode).toBe("no_net");
        capability.destroy();
    });

    it("reports unknown and empties the list when the config cannot be read", async () => {
        mockInvoke({
            api_get_config: () => {
                throw new Error("no api client");
            },
        });

        const capability = new BedrockCapabilityManager();
        await capability.refresh();

        expect(get(capability.status)).toBe("unknown");
        expect(get(capability.serverProvidedServers)).toHaveLength(0);
        capability.destroy();
    });
});
