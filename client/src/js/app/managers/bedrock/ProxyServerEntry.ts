export interface ProxyServerEntry {
    id: string;
    name: string;
    host: string;
    port: number;
    // Raw Bedrock protocol version to advertise to clients for this server.
    // Omitted means Auto — the proxy mirrors the real backend's version.
    protocolVersion?: number;
}
