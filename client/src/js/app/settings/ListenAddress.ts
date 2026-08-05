import type { NetworkInterface } from "../../bindings/NetworkInterface";

export interface ListenChoice {
    readonly id: string;
    readonly label: string;
    readonly bind: string;
}

/**
 * Where the proxy listens, and what to type into Minecraft.
 *
 * Two different addresses. They agree on a single-interface bind and diverge on
 * `0.0.0.0`, which nothing can connect to.
 */
export class ListenAddress {
    static readonly ANY = "0.0.0.0";
    static readonly LOOPBACK = "127.0.0.1";

    private static readonly LINK_LOCAL = "169.254.";

    static choices(interfaces: readonly NetworkInterface[]): readonly ListenChoice[] {
        const named = interfaces
            .filter((nic) => nic.is_ipv4)
            .map((nic) => ({ id: nic.ip, label: `${nic.ip} — ${nic.name}`, bind: nic.ip }));
        return [
            ...named,
            { id: "any", label: `${this.ANY} — every interface`, bind: this.ANY },
        ];
    }

    /** What you type into Minecraft on this machine. */
    static join(bind: string, port: number): string {
        return `${bind === this.ANY ? this.LOOPBACK : bind}:${port}`;
    }

    /** What another device types, or null when that is the same as `join`. */
    static lan(
        bind: string,
        port: number,
        interfaces: readonly NetworkInterface[],
    ): string | null {
        if (bind !== this.ANY) return null;
        const routable = interfaces.find(
            (nic) =>
                nic.is_ipv4 &&
                nic.ip !== this.LOOPBACK &&
                !nic.ip.startsWith("127.") &&
                !nic.ip.startsWith(this.LINK_LOCAL),
        );
        return routable ? `${routable.ip}:${port}` : null;
    }

    /**
     * Every address something else on the network could connect to, best first.
     *
     * Which interface is Wi-Fi, cellular, a VPN or a tethering bridge is not decidable from
     * an address: the interface name is the only hint and it is not a contract. So all the
     * plausible ones are listed and the reader picks. Private ranges rank first because
     * those are the ones a device on the same network can reach.
     */
    static candidates(
        interfaces: readonly NetworkInterface[],
        port: number,
        includeLoopback = false,
    ): readonly { readonly label: string; readonly address: string }[] {
        // Where the thing connecting may be on this machine — Minecraft usually is — the
        // loopback address leads. Nothing local drives the WebSocket server, so it opts out.
        const local = includeLoopback
            ? [{ label: "this device", address: `${this.LOOPBACK}:${port}` }]
            : [];
        return local.concat(
            interfaces
            .filter(
                (nic) =>
                    nic.is_ipv4 &&
                    nic.ip !== this.LOOPBACK &&
                    !nic.ip.startsWith("127.") &&
                    !nic.ip.startsWith(this.LINK_LOCAL),
            )
            .map((nic) => ({ nic, rank: this.rank(nic) }))
            .sort((a, b) => a.rank - b.rank)
            .map(({ nic }) => ({ label: nic.name, address: `${nic.ip}:${port}` })),
        );
    }

    /** Lower sorts first: private, then anything else, then a virtual interface. */
    private static rank(nic: NetworkInterface): number {
        const name = nic.name.toLowerCase();
        const virtual = ["tun", "tap", "utun", "wg", "zt", "docker", "veth", "vmnet", "vethernet", "hyper-v", "loopback"];
        if (virtual.some((token) => name.includes(token))) return 2;
        return this.isPrivate(nic.ip) ? 0 : 1;
    }

    /** RFC1918, which is what a device on the same network can reach. */
    static isPrivate(ip: string): boolean {
        if (ip.startsWith("192.168.") || ip.startsWith("10.")) return true;
        const second = Number(ip.split(".")[1]);
        return ip.startsWith("172.") && second >= 16 && second <= 31;
    }
}
