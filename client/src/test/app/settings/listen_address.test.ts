import { describe, expect, it } from "vitest";
import { ListenAddress } from "../../../js/app/settings/ListenAddress";
import type { NetworkInterface } from "../../../js/bindings/NetworkInterface";

const NICS: NetworkInterface[] = [
    { name: "Loopback", ip: "127.0.0.1", is_ipv4: true },
    { name: "Ethernet", ip: "192.168.1.24", is_ipv4: true },
    { name: "Wi-Fi", ip: "192.168.1.31", is_ipv4: true },
    { name: "Tailscale", ip: "fd7a::1", is_ipv4: false },
];

describe("ListenAddress.join", () => {
    it("is the bind address when the proxy is bound to one interface", () => {
        expect(ListenAddress.join("192.168.1.24", 19132)).toBe("192.168.1.24:19132");
    });

    // Nothing can connect to 0.0.0.0. On this machine the answer is loopback.
    it("is loopback when the proxy is bound to everything", () => {
        expect(ListenAddress.join("0.0.0.0", 19132)).toBe("127.0.0.1:19132");
    });
});

describe("ListenAddress.lan", () => {
    it("names the address another device would use when that differs", () => {
        expect(ListenAddress.lan("0.0.0.0", 19132, NICS)).toBe("192.168.1.24:19132");
    });

    // Bound to one interface there is only one answer, and a second sentence about
    // other devices is noise on the row.
    it("says nothing when the two answers are the same", () => {
        expect(ListenAddress.lan("192.168.1.24", 19132, NICS)).toBeNull();
        expect(ListenAddress.lan("127.0.0.1", 19132, NICS)).toBeNull();
    });

    it("says nothing when there is no routable address", () => {
        const alone: NetworkInterface[] = [{ name: "Loopback", ip: "127.0.0.1", is_ipv4: true }];
        expect(ListenAddress.lan("0.0.0.0", 19132, alone)).toBeNull();
    });

    // A link-local address means DHCP did not answer. Telling somebody to type it at
    // their console sends them after a connection that cannot be made.
    it("does not offer a link-local address", () => {
        const stranded: NetworkInterface[] = [
            { name: "Loopback", ip: "127.0.0.1", is_ipv4: true },
            { name: "Ethernet", ip: "169.254.11.4", is_ipv4: true },
        ];
        expect(ListenAddress.lan("0.0.0.0", 19132, stranded)).toBeNull();
    });
});

describe("ListenAddress.candidates", () => {
    // Bound to every interface, `0.0.0.0` is not something anything can connect to. Which
    // of the device's addresses is Wi-Fi, cellular or a VPN is not decidable, so all the
    // plausible ones are offered.
    it("lists every address something else could connect to", () => {
        const found = ListenAddress.candidates(NICS, 9595).map((c) => c.address);
        expect(found).toContain("192.168.1.24:9595");
        expect(found).toContain("192.168.1.31:9595");
    });

    it("leaves out loopback, link-local and IPv6", () => {
        const messy: NetworkInterface[] = [
            { name: "Loopback", ip: "127.0.0.1", is_ipv4: true },
            { name: "Ethernet", ip: "169.254.10.4", is_ipv4: true },
            { name: "Tailscale", ip: "fd7a::1", is_ipv4: false },
        ];
        expect(ListenAddress.candidates(messy, 9595)).toHaveLength(0);
    });

    // A private address is the one a device on the same network can reach, so it leads.
    it("ranks a private address above a public one", () => {
        const mixed: NetworkInterface[] = [
            { name: "rmnet0", ip: "100.82.4.9", is_ipv4: true },
            { name: "wlan0", ip: "192.168.1.24", is_ipv4: true },
        ];
        expect(ListenAddress.candidates(mixed, 9595)[0]?.address).toBe("192.168.1.24:9595");
    });

    // A VPN or a container bridge holds a private address too, and is almost never the
    // answer, so it sorts last rather than being hidden.
    it("sinks a virtual interface below a real one", () => {
        const mixed: NetworkInterface[] = [
            { name: "tun0", ip: "10.8.0.2", is_ipv4: true },
            { name: "wlan0", ip: "192.168.1.24", is_ipv4: true },
        ];
        const found = ListenAddress.candidates(mixed, 9595);
        expect(found[0]?.address).toBe("192.168.1.24:9595");
        expect(found[1]?.label).toBe("tun0");
    });

    it("names each one by its interface, which is the only hint available", () => {
        const found = ListenAddress.candidates(
            [{ name: "wlan0", ip: "192.168.1.24", is_ipv4: true }],
            9595,
        );
        expect(found[0]?.label).toBe("wlan0");
    });
});

describe("ListenAddress.isPrivate", () => {
    it("knows the RFC1918 ranges", () => {
        expect(ListenAddress.isPrivate("192.168.0.1")).toBe(true);
        expect(ListenAddress.isPrivate("10.1.2.3")).toBe(true);
        expect(ListenAddress.isPrivate("172.16.0.1")).toBe(true);
        expect(ListenAddress.isPrivate("172.31.255.254")).toBe(true);
    });

    // 172.32 is public, and treating the whole 172 block as private would rank a public
    // address as the one to hand out.
    it("does not claim the whole 172 block", () => {
        expect(ListenAddress.isPrivate("172.32.0.1")).toBe(false);
        expect(ListenAddress.isPrivate("172.15.0.1")).toBe(false);
    });
});
