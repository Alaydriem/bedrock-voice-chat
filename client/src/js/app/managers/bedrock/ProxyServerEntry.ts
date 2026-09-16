import type { AddonMode } from '../../../bindings/AddonMode';

export interface ProxyServerEntry {
    id: string;
    name: string;
    host: string;
    port: number;
    // Raw Bedrock protocol version to advertise to clients for this server.
    // Omitted means Auto — the proxy mirrors the real backend's version.
    protocolVersion?: number;
    // How this world's addon reaches the BVC server. Omitted resolves from the
    // advertised list, then from the known no-net hosts, and defaults to `net`
    // — the proxy relays only and does not carry the bvc: jukebox or control
    // buses. Turn it to `no_net` for any world whose addon cannot reach the
    // BVC server itself.
    addonMode?: AddonMode;
    // Present on entries advertised by the BVC server's config; they are
    // read-only and never persisted locally. Absent on user-created entries.
    source?: "server";
}
