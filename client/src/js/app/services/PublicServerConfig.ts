import { invoke } from "@tauri-apps/api/core";
import type { ApiConfigResponse } from "../../bindings/ApiConfigResponse";

/**
 * A server's `/api/config`, read without credentials.
 *
 * The one thing a BVC server answers to anybody. Two callers want it for different
 * reasons: the address field needs the QUIC ports before it can probe them, and a saved
 * server that just refused an mTLS call needs to know whether it is down or whether the
 * credentials are stale. Both are the same request, so it lives in one place.
 *
 * The request is made in Rust, not by the webview's HTTP plugin: only the Rust client
 * validates a play-zone server's certificate. The timeout lives there too, and is
 * `NetTimeouts::HTTPS` in `common/src/net/timeouts.rs`.
 */
export class PublicServerConfig {
    static async read(server: string): Promise<ApiConfigResponse> {
        return await invoke<ApiConfigResponse>("read_public_server_config", { server });
    }
}
