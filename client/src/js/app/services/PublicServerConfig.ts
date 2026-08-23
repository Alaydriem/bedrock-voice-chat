import { fetch } from '@tauri-apps/plugin-http';
import type { ApiConfigResponse } from '../../bindings/ApiConfigResponse';

/**
 * A server's `/api/config`, read without credentials.
 *
 * The one thing a BVC server answers to anybody. Two callers want it for different
 * reasons: the address field needs the QUIC ports before it can probe them, and a saved
 * server that just refused an mTLS call needs to know whether it is down or whether the
 * credentials are stale. Both are the same request, so it lives in one place.
 */
export class PublicServerConfig {
    /**
     * Mirrors `NetTimeouts::HTTPS` in `common/src/net/timeouts.rs`, which is the source of
     * this number and carries the reasoning. Hand-copied because ts-rs exports types and not
     * constants; if that one moves, this moves with it.
     */
    private static readonly TIMEOUT_MS = 7_000;

    /**
     * A cleared timer rather than AbortSignal.timeout: the Tauri http plugin never removes
     * its abort listener, so a timeout firing after a completed request cancels a resource
     * id that has already been dropped — which is the invalid resource id error that once
     * flooded Sentry.
     */
    static async read(server: string): Promise<ApiConfigResponse> {
        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), PublicServerConfig.TIMEOUT_MS);
        try {
            const response = await fetch(`${server}/api/config`, {
                method: 'GET',
                signal: controller.signal,
            });
            if (response.status !== 200) throw new Error(`config returned ${response.status}`);
            return (await response.json()) as ApiConfigResponse;
        } finally {
            clearTimeout(timer);
        }
    }
}
