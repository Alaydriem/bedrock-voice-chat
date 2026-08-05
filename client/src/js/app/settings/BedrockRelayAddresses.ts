/** One way in that the BVC server offers, rather than one this machine has. */
export interface RelayAddress {
    readonly label: string;
    readonly address: string;
    readonly note: string;
    /** False for something joined from Minecraft's featured servers, not typed in. */
    readonly typed: boolean;
}

export interface RelayOffer {
    /** Host of the connected BVC server, without a scheme. */
    readonly host: string;
    /** The transfer relay's port, or null when the server does not run one. */
    readonly transferPort: number | null;
    /** The host the DNS override answers, or null when DNS is off. */
    readonly dnsOverrideHost: string | null;
}

/**
 * The ways into a Bedrock session that come from the server rather than from this
 * machine.
 *
 * The transfer relay is a Bedrock server on the BVC host that moves a joining player to
 * whichever backend they are set to. The DNS override answers a featured server's name
 * with the BVC host, so a player who redirects their device's DNS reaches the same relay
 * without adding a server at all.
 */
export class BedrockRelayAddresses {
    static list(offer: RelayOffer): readonly RelayAddress[] {
        const host = offer.host.trim();
        if (!host) return [];

        const addresses: RelayAddress[] = [];

        if (offer.transferPort !== null) {
            addresses.push({
                label: "Transfer server",
                address: `${host}:${offer.transferPort}`,
                note: "Add this in Minecraft. It sends you on to the server you picked above.",
                typed: true,
            });
        }

        // Only alongside a relay: the override sends a player to the transfer port, and
        // pointing DNS at a server with nothing listening there breaks the featured
        // server it answers for.
        if (offer.dnsOverrideHost && offer.transferPort !== null) {
            addresses.push({
                label: "DNS override",
                address: offer.dnsOverrideHost,
                note: `Point your device's DNS at ${host}, then join this from Minecraft's featured servers. Nothing to add by hand.`,
                typed: false,
            });
        }

        return addresses;
    }
}
