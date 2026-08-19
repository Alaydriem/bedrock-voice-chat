export interface WebSocketConfig {
    /** Retained so a config written before the server became always-on still round-trips. */
    enabled: boolean;
    /** Retained for the one-time migration to `allow_external`. */
    localhost_only: boolean;
    allow_external: boolean;
    port: number;
    key: string;
}
