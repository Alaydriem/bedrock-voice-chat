import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { LoginResponse } from '../../bindings/LoginResponse';
import type { ApiConfigCheckResponse } from '../../bindings/ApiConfigCheckResponse';
import type { ServerHealthResult } from './ServerHealthResult';

export class ServerHealthService {
    async check(server: string): Promise<ServerHealthResult> {
        let credentials: LoginResponse;
        try {
            credentials = await invoke<LoginResponse>('get_credentials', { server });
        } catch (e) {
            logError(`get_credentials failed for ${server}: ${e}`);
            return {
                status: 'missing',
                compatible: false,
                clientTooOld: false,
                serverVersion: '',
                clientVersion: '',
            };
        }

        const expired = await invoke<boolean>('is_certificate_expired', { server });
        if (expired) {
            return {
                status: 'reauth',
                compatible: false,
                clientTooOld: false,
                serverVersion: '',
                clientVersion: '',
            };
        }

        await invoke('api_initialize_client', {
            endpoint: server,
            cert: credentials.certificate_ca,
            pem: credentials.certificate + credentials.certificate_key,
        });

        try {
            await invoke('refresh_server_state', { server });
        } catch (e) {
            logError(`refresh_server_state non-fatal for ${server}: ${e}`);
        }

        const configResponse = await invoke<ApiConfigCheckResponse>('api_get_config', { server });

        if (!configResponse.compatible) {
            return {
                status: 'version_mismatch',
                compatible: false,
                clientTooOld: configResponse.client_too_old,
                serverVersion: configResponse.config.protocol_version,
                clientVersion: configResponse.client_version,
            };
        }

        return {
            status: 'connect',
            compatible: true,
            clientTooOld: false,
            serverVersion: configResponse.config.protocol_version,
            clientVersion: configResponse.client_version,
        };
    }
}
