import { invoke } from '@tauri-apps/api/core';
import type { ApiConfigResponse } from '../../bindings/ApiConfigResponse';

/**
 * The availability check the sign-in screen gates on.
 *
 * Routed through Rust rather than the webview's HTTP plugin so the check and the sign-in
 * that follows it are made over one client configuration: same TLS, same address family,
 * same connect budget. A check made over different settings can answer for a server the
 * sign-in cannot reach.
 */
export class ServerCheck {
    public static async config(server: string): Promise<ApiConfigResponse> {
        return await invoke<ApiConfigResponse>('check_server', { server });
    }
}
