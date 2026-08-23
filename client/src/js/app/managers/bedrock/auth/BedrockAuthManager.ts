import { writable, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { openUrl } from '@tauri-apps/plugin-opener';
import { info, error as logError } from '@charlesportwoodii/tauri-plugin-curia';
import type { BedrockAuthManagerCallbacks } from './BedrockAuthManagerCallbacks';

export class BedrockAuthManager {
    private isAuthenticatedStore: Writable<boolean>;
    private isRestoringAuthStore: Writable<boolean>;
    private showLoginModalStore: Writable<boolean>;
    private deviceCodeStore: Writable<string>;
    private deviceUrlStore: Writable<string>;
    private loginErrorStore: Writable<string>;
    private codeCopiedStore: Writable<boolean>;

    public readonly isAuthenticated: Readable<boolean>;
    public readonly isRestoringAuth: Readable<boolean>;
    public readonly showLoginModal: Readable<boolean>;
    public readonly deviceCode: Readable<string>;
    public readonly deviceUrl: Readable<string>;
    public readonly loginError: Readable<string>;
    public readonly codeCopied: Readable<boolean>;

    private loginFlowUnlisten: (() => void) | null = null;
    private copiedTimeout: ReturnType<typeof setTimeout> | null = null;
    private callbacks: BedrockAuthManagerCallbacks;

    constructor(callbacks: BedrockAuthManagerCallbacks) {
        this.callbacks = callbacks;

        this.isAuthenticatedStore = writable(false);
        this.isRestoringAuthStore = writable(true);
        this.showLoginModalStore = writable(false);
        this.deviceCodeStore = writable('');
        this.deviceUrlStore = writable('');
        this.loginErrorStore = writable('');
        this.codeCopiedStore = writable(false);

        this.isAuthenticated = { subscribe: this.isAuthenticatedStore.subscribe };
        this.isRestoringAuth = { subscribe: this.isRestoringAuthStore.subscribe };
        this.showLoginModal = { subscribe: this.showLoginModalStore.subscribe };
        this.deviceCode = { subscribe: this.deviceCodeStore.subscribe };
        this.deviceUrl = { subscribe: this.deviceUrlStore.subscribe };
        this.loginError = { subscribe: this.loginErrorStore.subscribe };
        this.codeCopied = { subscribe: this.codeCopiedStore.subscribe };
    }

    isAuthenticatedNow(): boolean {
        return get(this.isAuthenticatedStore);
    }

    async restoreAuth(): Promise<void> {
        try {
            const restored = await invoke<boolean>('bedrock_restore_auth');
            if (restored) {
                this.isAuthenticatedStore.set(true);
            }
        } catch (e) {
            logError(`Auth restore failed: ${e}`);
        }
    }

    setAuthenticated(value: boolean): void {
        this.isAuthenticatedStore.set(value);
    }

    finishRestoring(): void {
        this.isRestoringAuthStore.set(false);
    }

    async openLoginModal(): Promise<void> {
        this.deviceCodeStore.set('');
        this.deviceUrlStore.set('');
        this.loginErrorStore.set('');
        this.showLoginModalStore.set(true);

        const appWebview = getCurrentWebviewWindow();
        this.loginFlowUnlisten = await appWebview.listen(
            'bedrock-device-code',
            (event: { payload?: { code?: string; url?: string } }) => {
                const payload = event.payload;
                if (payload?.code) {
                    this.deviceCodeStore.set(payload.code);
                }
                if (payload?.url) {
                    this.deviceUrlStore.set(payload.url);
                }
                info(`Device code received: ${payload?.code}`);
            },
        );

        try {
            await invoke('bedrock_xbox_login');
            info('Xbox login succeeded');
            this.isAuthenticatedStore.set(true);
            this.callbacks.setStatus('Signed in to Xbox Live');
            this.showLoginModalStore.set(false);
            this.cleanupLoginListener();
            await this.callbacks.onLoginSuccess();
        } catch (e) {
            const msg = String(e);
            if (msg === 'Login cancelled') {
                this.showLoginModalStore.set(false);
            } else {
                this.loginErrorStore.set(msg);
                logError(`Xbox login failed: ${msg}`);
            }
            this.cleanupLoginListener();
        }
    }

    async closeLoginModal(): Promise<void> {
        try {
            await invoke('bedrock_cancel_xbox_login');
        } catch (e) {
            logError(`Cancel failed: ${e}`);
        }
        this.cleanupLoginListener();
        this.showLoginModalStore.set(false);
    }

    async signOut(): Promise<void> {
        try {
            await invoke('bedrock_xbox_logout');
            this.isAuthenticatedStore.set(false);
            this.callbacks.setStatus('');
        } catch (e) {
            this.callbacks.setStatus(`Error: ${e}`);
        }
    }

    async copyDeviceCode(): Promise<void> {
        try {
            await navigator.clipboard.writeText(get(this.deviceCodeStore));
            this.codeCopiedStore.set(true);
            if (this.copiedTimeout) {
                clearTimeout(this.copiedTimeout);
            }
            this.copiedTimeout = setTimeout(() => {
                this.codeCopiedStore.set(false);
                this.copiedTimeout = null;
            }, 2000);
        } catch (e) {
            logError(`Clipboard write failed: ${e}`);
        }
    }

    async openLoginUrl(): Promise<void> {
        try {
            await openUrl(get(this.deviceUrlStore));
        } catch (e) {
            logError(`Failed to open URL: ${e}`);
        }
    }

    private cleanupLoginListener(): void {
        if (this.loginFlowUnlisten) {
            this.loginFlowUnlisten();
            this.loginFlowUnlisten = null;
        }
    }

    destroy(): void {
        this.cleanupLoginListener();
        if (this.copiedTimeout) {
            clearTimeout(this.copiedTimeout);
        }
    }
}
