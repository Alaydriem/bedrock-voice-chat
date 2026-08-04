import { invoke } from '@tauri-apps/api/core';
import { error as logError, warn } from '@tauri-apps/plugin-log';
import type { LoginResponse } from '../../bindings/LoginResponse';
import type { ApiConfigCheckResponse } from '../../bindings/ApiConfigCheckResponse';
import type { ServerHealthResult } from './ServerHealthResult';
import type { ServerHealthStatus } from './ServerHealthStatus';
import { PublicServerConfig } from './PublicServerConfig';

/**
 * Whether one saved server can be joined right now, and if not, why not.
 *
 * Read-only with respect to the session: the client it builds goes into the pool without
 * becoming the current server, so checking every saved server in parallel cannot leave
 * `current_server` pointing at whichever check finished last. Signing in claims a server,
 * and the dashboard does that on entry.
 *
 * Never throws. Every failure is one of the statuses, because a row in a list has to say
 * something and "checking" forever is not an answer.
 */
export class ServerHealthService {
    async check(server: string): Promise<ServerHealthResult> {
        let credentials: LoginResponse;
        try {
            credentials = await invoke<LoginResponse>('get_credentials', { server });
        } catch (e) {
            logError(`get_credentials failed for ${server}: ${e}`);
            return ServerHealthService.verdict('missing');
        }

        try {
            if (await invoke<boolean>('is_certificate_expired', { server })) {
                return ServerHealthService.verdict('reauth');
            }
        } catch (e) {
            warn(`Could not read the certificate for ${server}: ${e}`);
            return ServerHealthService.verdict('reauth');
        }

        try {
            await invoke('api_pool_client', {
                endpoint: server,
                cert: credentials.certificate_ca,
                pem: credentials.certificate + credentials.certificate_key,
            });

            // No `refresh_server_state` here. It rotates the certificate and writes the new
            // one to the keyring, and looking at a list of servers is not a reason to
            // rotate credentials for every server on it. Connecting does that, for the one
            // that was chosen.
            const response = await invoke<ApiConfigCheckResponse>('api_get_config', { server });

            if (!response.compatible) {
                return {
                    status: 'version_mismatch',
                    compatible: false,
                    clientTooOld: response.client_too_old,
                    serverVersion: response.config.protocol_version,
                    clientVersion: response.client_version,
                };
            }

            return {
                status: 'connect',
                compatible: true,
                clientTooOld: false,
                serverVersion: response.config.protocol_version,
                clientVersion: response.client_version,
            };
        } catch (e) {
            warn(`Authenticated config read failed for ${server}: ${e}`);
            return ServerHealthService.verdict(await ServerHealthService.whyItFailed(server));
        }
    }

    /**
     * An mTLS call failed. That is either the server being down or the credentials no
     * longer being accepted, and the two lead somewhere different, so ask the server the
     * one question it answers to anybody.
     */
    private static async whyItFailed(server: string): Promise<ServerHealthStatus> {
        return (await PublicServerConfig.isAnswering(server)) ? 'reauth' : 'unreachable';
    }

    private static verdict(status: ServerHealthStatus): ServerHealthResult {
        return {
            status,
            compatible: false,
            clientTooOld: false,
            serverVersion: '',
            clientVersion: '',
        };
    }
}
