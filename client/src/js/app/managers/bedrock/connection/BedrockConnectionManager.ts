import { writable, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { error as logError } from '@tauri-apps/plugin-log';
import type { BedrockConnectError } from '../../../../bindings/BedrockConnectError';
import type { BedrockConnectionInfo } from '../../../../bindings/BedrockConnectionInfo';
import { BedrockConnectErrorMapper } from './BedrockConnectErrorMapper';
import type { RealmsConnectionError } from './RealmsConnectionError';
import type { RealmsLifecycle } from '../realms/RealmsLifecycle';

export class BedrockConnectionManager {
    private connectionErrorStore: Writable<RealmsConnectionError | null>;
    private connectionInfoStore: Writable<BedrockConnectionInfo | null>;
    private connectErrorUnlisten: (() => void) | null = null;
    private connectionInfoUnlisten: (() => void) | null = null;
    private readonly getRealmsLifecycle: () => RealmsLifecycle | null;
    private onReauthRequired: (() => void) | null = null;

    public readonly connectionError: Readable<RealmsConnectionError | null>;
    public readonly connectionInfo: Readable<BedrockConnectionInfo | null>;

    constructor(getRealmsLifecycle: () => RealmsLifecycle | null) {
        this.getRealmsLifecycle = getRealmsLifecycle;
        this.connectionErrorStore = writable(null);
        this.connectionInfoStore = writable(null);
        this.connectionError = { subscribe: this.connectionErrorStore.subscribe };
        this.connectionInfo = { subscribe: this.connectionInfoStore.subscribe };
    }

    /**
     * Registers what to do when the backend reports the Xbox credential was rejected.
     *
     * Set rather than injected because the handler raises the sign-in modal, which the auth
     * manager owns, and this manager is constructed before it.
     */
    setReauthHandler(handler: () => void): void {
        this.onReauthRequired = handler;
    }

    async initialize(): Promise<void> {
        await this.subscribeToConnectErrors();
        await this.subscribeToConnectionInfo();
    }

    private async subscribeToConnectErrors(): Promise<void> {
        if (this.connectErrorUnlisten) {
            return;
        }
        const appWebview = getCurrentWebviewWindow();
        this.connectErrorUnlisten = await appWebview.listen<BedrockConnectError>(
            'bedrock-connect-error',
            (event) => {
                if (get(this.connectionErrorStore)) {
                    return;
                }
                this.connectionErrorStore.set(BedrockConnectErrorMapper.describe(event.payload));

                // A rejected credential cannot be repaired by the teardown's refresh, and
                // running one would spend the player's next sign-in attempt on a token that
                // is already dead.
                if (event.payload.kind === 'reauth_required') {
                    this.onReauthRequired?.();
                    return;
                }

                void this.autoTeardownAfterError();
            },
        );
    }

    private async subscribeToConnectionInfo(): Promise<void> {
        if (this.connectionInfoUnlisten) {
            return;
        }
        const appWebview = getCurrentWebviewWindow();
        this.connectionInfoUnlisten = await appWebview.listen<BedrockConnectionInfo>(
            'bedrock_connection_info',
            (event) => {
                this.connectionInfoStore.set(event.payload);
            },
        );
    }

    setConnectErrorFromInvoke(raw: string): void {
        if (get(this.connectionErrorStore)) {
            return;
        }
        this.connectionErrorStore.set(
            BedrockConnectErrorMapper.describe({ kind: 'other', message: raw }),
        );
        void this.autoTeardownAfterError();
    }

    clearError(): void {
        this.connectionErrorStore.set(null);
    }

    dismissConnectionError(): void {
        this.connectionErrorStore.set(null);
    }

    dismissConnectionInfo(): void {
        this.connectionInfoStore.set(null);
    }

    hasError(): boolean {
        return get(this.connectionErrorStore) !== null;
    }

    private async autoTeardownAfterError(): Promise<void> {
        const realms = this.getRealmsLifecycle();
        if (realms && realms.isRunning()) {
            try {
                await realms.stopRealms();
            } catch (e) {
                logError(`Auto stopRealms after detected error failed: ${e}`);
            }
        }
    }

    destroy(): void {
        if (this.connectErrorUnlisten) {
            this.connectErrorUnlisten();
            this.connectErrorUnlisten = null;
        }
        if (this.connectionInfoUnlisten) {
            this.connectionInfoUnlisten();
            this.connectionInfoUnlisten = null;
        }
    }
}
