import { describe, expect, it } from "vitest";
import { BedrockRelayAddresses } from "../../../js/app/settings/BedrockRelayAddresses";

const HOST = "bvc.example.com";

describe("BedrockRelayAddresses", () => {
    it("offers the transfer relay on the BVC host", () => {
        const list = BedrockRelayAddresses.list({
            host: HOST,
            transferPort: 19132,
            dnsOverrideHost: null,
        });
        expect(list).toHaveLength(1);
        expect(list[0]?.address).toBe("bvc.example.com:19132");
        expect(list[0]?.typed).toBe(true);
    });

    // A server without the relay compiled in or turned on has nothing to send anyone to,
    // so an entry here would be an address that refuses the connection.
    it("offers nothing when the server runs no relay", () => {
        expect(
            BedrockRelayAddresses.list({ host: HOST, transferPort: null, dnsOverrideHost: null }),
        ).toEqual([]);
    });

    it("adds the DNS override beside the relay", () => {
        const list = BedrockRelayAddresses.list({
            host: HOST,
            transferPort: 19132,
            dnsOverrideHost: "geo.hivebedrock.network",
        });
        expect(list.map((a) => a.address)).toEqual([
            "bvc.example.com:19132",
            "geo.hivebedrock.network",
        ]);
    });

    // The override answers a featured server's name with this host, and the player lands
    // on the transfer port. Named without the relay, it would break the featured server it
    // answers for.
    it("withholds the DNS override when there is no relay behind it", () => {
        expect(
            BedrockRelayAddresses.list({
                host: HOST,
                transferPort: null,
                dnsOverrideHost: "geo.hivebedrock.network",
            }),
        ).toEqual([]);
    });

    // It is not added in Minecraft, so a note telling somebody to add it sends them to a
    // dead end. The note names the DNS change instead.
    it("says the override is reached by redirecting DNS, not by adding a server", () => {
        const list = BedrockRelayAddresses.list({
            host: HOST,
            transferPort: 19132,
            dnsOverrideHost: "geo.hivebedrock.network",
        });
        const override = list[1];
        expect(override?.typed).toBe(false);
        expect(override?.note).toContain("DNS");
        expect(override?.note).toContain(HOST);
    });

    // The host is read from the store, so it is empty until that read lands. An entry
    // reading ":19132" is not an address.
    it("offers nothing until the BVC host is known", () => {
        expect(
            BedrockRelayAddresses.list({ host: "", transferPort: 19132, dnsOverrideHost: "h" }),
        ).toEqual([]);
        expect(
            BedrockRelayAddresses.list({ host: "  ", transferPort: 19132, dnsOverrideHost: "h" }),
        ).toEqual([]);
    });
});
