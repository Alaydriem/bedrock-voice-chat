import type { AddonTransport } from '../../../bindings/AddonTransport';

export interface ProxyServerEntry {
    id: string;
    name: string;
    host: string;
    port: number;
    // Raw Bedrock protocol version to advertise to clients for this server.
    // Omitted means Auto — the proxy mirrors the real backend's version.
    protocolVersion?: number;
    // How this world's addon reaches the BVC server. Omitted resolves from the
    // advertised list, then defaults to no-net.
    addonTransport?: AddonTransport;
    // Present on entries advertised by the BVC server's config; they are
    // read-only and never persisted locally. Absent on user-created entries.
    source?: "server";
}
