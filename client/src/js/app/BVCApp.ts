import App from './app.js';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { info, warn, error as logError } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { Store } from '@tauri-apps/plugin-store';
import { DeepLinkRouter } from './deepLinkRouter.ts';
import type { DeepLink } from '../bindings/DeepLink';
import type { ConnectionHealth } from '../bindings/ConnectionHealth';

/**
 * Audio stream recovery event payload from the Rust backend
 */
interface AudioStreamRecoveryPayload {
    device_type: 'InputDevice' | 'OutputDevice';
    error: string;
}

export default class BVCApp extends App {
    private deepLinkRouter: DeepLinkRouter | null = null;
    private deepLinkUnlisten: UnlistenFn | null = null;
    private connectionHealthUnlisten: UnlistenFn | null = null;
    private audioRecoveryUnlisten: UnlistenFn | null = null;
    private initialized = false;
    private storeInstance: Store | null = null;

    constructor() {
        super();
        this.setupDeepLinkListener();
        this.setupConnectionHealthListener();
        this.setupAudioRecoveryListener();
    }

    /**
     * Get or create Store instance (cached)
     */
    protected async getStore(): Promise<Store> {
        if (!this.storeInstance) {
            this.storeInstance = await Store.load("store.json", {
                autoSave: false,
                defaults: {}
            });
        }
        return this.storeInstance;
    }

    /**
     * Synchronously register the Tauri event listener for deep links
     */
    private setupDeepLinkListener(): void {
        listen<DeepLink>('deep-link-received', (event) => {
            info(`BVCApp: Received deep-link-received event: ${event.payload.url}`);
            this.handleDeepLinkEvent(event.payload.url).catch((err) => {
                logError(`BVCApp: Failed to handle deep link event: ${err}`);
            });
        }).then((unlisten) => {
            this.deepLinkUnlisten = unlisten;
            info('BVCApp: deep-link-received listener registered');
        }).catch((err) => {
            logError(`BVCApp: Failed to register deep link listener: ${err}`);
        });
    }

    /**
     * Synchronously register the connection health listener for version mismatch handling
     */
    private setupConnectionHealthListener(): void {
        listen<ConnectionHealth>('connection_health', (event) => {
            if (event.payload.status === 'VersionMismatch') {
                const payload = event.payload as { status: 'VersionMismatch', client_version: string, server_version: string, client_too_old: boolean };
                const errorCode = payload.client_too_old ? 'VER01' : 'VER02';
                warn(`BVCApp: Version mismatch detected: client=${payload.client_version}, server=${payload.server_version}, redirecting to ${errorCode}`);
                window.location.href = `/error?code=${errorCode}`;
            }
        }).then((unlisten) => {
            this.connectionHealthUnlisten = unlisten;
            info('BVCApp: connection_health listener registered');
        }).catch((err) => {
            logError(`BVCApp: Failed to register connection health listener: ${err}`);
        });
    }

    /**
     * Synchronously register the audio recovery listener
     * This handler deals with audio device disconnect events to gracefully restart just the audio device if
     * is disconnected or fails unexpectedly.
     */
    private setupAudioRecoveryListener(): void {
        listen<AudioStreamRecoveryPayload>('audio-stream-recovery', async (event) => {
            const { device_type, error: streamError } = event.payload;
            warn(`BVCApp: Audio stream error on ${device_type}: ${streamError}`);

            // Brief delay before restart attempt to allow device state to settle
            await new Promise(resolve => setTimeout(resolve, 500));

            try {
                info(`BVCApp: Attempting to restart ${device_type} stream...`);
                await invoke('restart_audio_stream', { device: device_type });
                info(`BVCApp: ${device_type} recovered successfully`);
            } catch (e) {
                logError(`BVCApp: Audio recovery failed for ${device_type}: ${e}`);
            }
        }).then((unlisten) => {
            this.audioRecoveryUnlisten = unlisten;
            info('BVCApp: audio-stream-recovery listener registered');
        }).catch((err) => {
            logError(`BVCApp: Failed to register audio recovery listener: ${err}`);
        });
    }

    /**
     * Handle deep link event asynchronously
     */
    private async handleDeepLinkEvent(url: string): Promise<void> {
        try {
            // Lazy load router with injected store
            if (!this.deepLinkRouter) {
                const store = await this.getStore();
                this.deepLinkRouter = new DeepLinkRouter(store);
            }

            await this.deepLinkRouter.route(url);
        } catch (err) {
            logError(`BVCApp: Error routing deep link: ${err}`);
            if (this.deepLinkRouter) {
                await this.deepLinkRouter.clearPending();
            }
        }
    }

    /**
     * Public method to check for pending deep links
     * Called from page's initialize() method
     */
    async initializeDeepLinks(): Promise<void> {
        if (this.initialized) {
            info('BVCApp: Deep links already initialized');
            return;
        }

        info('BVCApp: Initializing deep links, checking for pending');

        try {
            // Create router with cached store
            if (!this.deepLinkRouter) {
                const store = await this.getStore();
                this.deepLinkRouter = new DeepLinkRouter(store);
            }

            await this.deepLinkRouter.processPending();
        } catch (err) {
            logError(`BVCApp: Error processing pending deep link: ${err}`);
            if (this.deepLinkRouter) {
                await this.deepLinkRouter.clearPending();
            }
        }

        this.initialized = true;
    }

    /**
     * Cleanup listeners
     */
    async cleanup(): Promise<void> {
        if (this.deepLinkUnlisten) {
            this.deepLinkUnlisten();
            this.deepLinkUnlisten = null;
        }
        if (this.connectionHealthUnlisten) {
            this.connectionHealthUnlisten();
            this.connectionHealthUnlisten = null;
        }
        if (this.audioRecoveryUnlisten) {
            this.audioRecoveryUnlisten();
            this.audioRecoveryUnlisten = null;
        }
    }
}
