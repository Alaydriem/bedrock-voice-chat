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
    /**
     * One router for the whole context, so the de-dup that protects a single-use OAuth
     * code is shared by every manager. A screen can construct more than one — the login
     * page has both `Login` and `LoginCode` — and a router per manager means a redemption
     * one of them made is invisible to the other.
     */
    private static deepLinkRouterPromise: Promise<DeepLinkRouter> | null = null;

    /**
     * Registered once per context, for the same reason. Two managers listening delivers
     * one deep link twice, and each delivery attempts its own redemption.
     */
    private static deepLinkRegistered = false;

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
        if (BVCApp.deepLinkRegistered) return;
        BVCApp.deepLinkRegistered = true;

        listen<DeepLink>('deep-link-received', (event) => {
            info(`BVCApp: Received deep-link-received event: ${event.payload.url.split(/[?#]/)[0]}`);
            this.handleDeepLinkEvent(event.payload.url).catch((err) => {
                logError(`BVCApp: Failed to handle deep link event: ${err}`);
            });
        }).then((unlisten) => {
            this.deepLinkUnlisten = unlisten;
            info('BVCApp: deep-link-received listener registered');
        }).catch((err) => {
            // Registration is claimed before the await, so a failure has to release the
            // claim or nothing would ever listen for deep links again.
            BVCApp.deepLinkRegistered = false;
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
            if (event.payload.status === 'Unauthorized') {
                const payload = event.payload as { status: 'Unauthorized', reason: string };
                warn(`BVCApp: Server refused the connection identity: ${payload.reason}`);
                window.location.href = '/error?code=AUTH01';
            }
        }).then((unlisten) => {
            this.connectionHealthUnlisten = unlisten;
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
     * Get or create the deep link router. One shared promise ensures the live
     * `deep-link-received` path, `initializeDeepLinks()`, and every manager in the
     * context use the same router, so its de-dup applies across all of them.
     */
    private getRouter(): Promise<DeepLinkRouter> {
        if (!BVCApp.deepLinkRouterPromise) {
            BVCApp.deepLinkRouterPromise = (async () => {
                const store = await this.getStore();
                return new DeepLinkRouter(store);
            })();
        }
        return BVCApp.deepLinkRouterPromise;
    }

    /**
     * Handle deep link event asynchronously
     */
    private async handleDeepLinkEvent(url: string): Promise<void> {
        try {
            const router = await this.getRouter();
            await router.route(url);
        } catch (err) {
            logError(`BVCApp: Error routing deep link: ${err}`);
            try {
                const router = await this.getRouter();
                await router.clearPending();
            } catch (clearErr) {
                logError(`BVCApp: Failed to clear pending deep link: ${clearErr}`);
            }
        }
    }

    /**
     * Process any pending deep link. Called from a page's initialize().
     * Returns true when a pending link was routed (the handler is navigating),
     * so the caller can skip its own navigation rather than override it.
     */
    async initializeDeepLinks(): Promise<boolean> {
        if (this.initialized) {
            info('BVCApp: Deep links already initialized');
            return false;
        }
        this.initialized = true;

        info('BVCApp: Initializing deep links, checking for pending');

        try {
            const router = await this.getRouter();
            return await router.processPending();
        } catch (err) {
            logError(`BVCApp: Error processing pending deep link: ${err}`);
            try {
                const router = await this.getRouter();
                await router.clearPending();
            } catch (clearErr) {
                logError(`BVCApp: Failed to clear pending deep link: ${clearErr}`);
            }
            return false;
        }
    }

    /**
     * Cleanup listeners
     */
    async cleanup(): Promise<void> {
        if (this.deepLinkUnlisten) {
            this.deepLinkUnlisten();
            this.deepLinkUnlisten = null;
            // Only the instance that registered holds the unlisten, and tearing it down
            // removes the context's only deep-link listener. Releasing the claim lets the
            // next manager register one rather than leaving deep links unhandled.
            BVCApp.deepLinkRegistered = false;
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

    async shutdown(): Promise<void> {
        try {
            const recording = await invoke<boolean>("is_recording");
            if (recording) {
                await invoke("stop_recording");
            }
        } catch (err) {
            logError(`Error stopping recording during shutdown: ${err}`);
        }

        await this.cleanup();
        await invoke("reset_asm");
        await invoke("reset_nsm");
    }
}
