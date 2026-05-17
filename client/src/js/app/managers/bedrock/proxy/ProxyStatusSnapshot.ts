export interface ProxyStatusSnapshot {
    host: string | null;
    port: number | null;
    listenPort: number | null;
    running: boolean;
}
