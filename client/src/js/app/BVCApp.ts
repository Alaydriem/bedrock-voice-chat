import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { info, warn, error as logError } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { Store } from '@tauri-apps/plugin-store';
import { DeepLinkRouter } from './deepLinkRouter.ts';
import { AppStore } from './services/AppStore';
import LocaleManager from './managers/settings/LocaleManager';
import BootOverlay from './shell/BootOverlay';
import { BootTimeline } from './shell/BootTimeline';
import type { DeepLink } from '../bindings/DeepLink';
import type { ConnectionHealth } from '../bindings/ConnectionHealth';

/**
 * Audio stream recovery event payload from the Rust backend
 */
interface AudioStreamRecoveryPayload {
    device_type: 'InputDevice' | 'OutputDevice';
    error: string;
}

export default class BVCApp {
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
    /**
     * One language pack for the whole context, for the same reason as the router above.
     * Settings reads it to render the picker and writes it to change the language; a
     * manager per caller would leave the screen showing a language the app is not in.
     */
    private static localeManagerPromise: Promise<LocaleManager> | null = null;

    private storeInstance: Store | null = null;
    private isCleanedUp = false;

    constructor() {
        this.registerCleanupEvents();
        this.setupDeepLinkListener();
        this.setupConnectionHealthListener();
        this.setupAudioRecoveryListener();
    }

    /**
     * Take the boot overlay down, once a screen is up behind it.
     *
     * Every route that renders a first screen calls this. False means there was nothing
     * to dismiss, which is the normal answer on a client-side navigation.
     */
    preloader(): boolean {
        return BootOverlay.dismiss();
    }

    /**
     * The listeners this instance registered are process-wide, so they outlive the
     * document unless something releases them. `pagehide` covers the webview cases
     * `beforeunload` misses on mobile.
     */
    private registerCleanupEvents(): void {
        const cleanup = () => this.safeCleanup();
        window.addEventListener('beforeunload', cleanup);
        window.addEventListener('pagehide', cleanup);
        window.addEventListener('popstate', cleanup);
        window.addEventListener('unload', cleanup);
    }

    private safeCleanup(): void {
        if (this.isCleanedUp) return;
        this.isCleanedUp = true;
        this.cleanup().catch((err) => logError(`BVCApp: Error during cleanup: ${err}`));
    }

    /**
     * Get or create Store instance (cached)
     */
    protected async getStore(): Promise<Store> {
        if (!this.storeInstance) {
            this.storeInstance = await AppStore.load();
        }
        return this.storeInstance;
    }

    /**
     * The language pack, resolved before the first screen renders.
     *
     * Shared as a promise rather than a manager so concurrent callers during startup
     * queue on one in-flight load instead of racing to start several.
     */
    static localeManager(): Promise<LocaleManager> {
        BVCApp.localeManagerPromise ??= (async () => {
            const manager = new LocaleManager(await AppStore.load());
            await manager.initialize();
            return manager;
        })();
        return BVCApp.localeManagerPromise;
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

        // Ahead of any navigation: a pack adopted after first paint repaints every
        // translated string, which reads as a flash of the wrong language.
        await BVCApp.localeManager();

        try {
            const router = await this.getRouter();
            // The first `Store.load` of the launch, and the first IPC of any kind. Marked apart
            // from the routing that follows because the two have nothing to do with each other,
            // and only one of them is likely to be worth attacking.
            BootTimeline.shared().mark('first Store.load (plugin init)');
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
     * Route a callback that is already sitting in the store.
     *
     * `initializeDeepLinks` is one-shot by design — it is the boot check. This is the same work
     * without that guard, for a caller that knows it is waiting for a callback and cannot assume
     * anything will announce it.
     *
     * Every intent writes `pending_deep_link` *and* emits `deep-link-received`, and the emit is
     * the only half anything listens to while a page is already open. If it is missed — fired
     * before the listener was registered, or after a teardown released it — the entry sits in the
     * store with nothing to consume it, and the screen waits for an event that has already been
     * and gone. Asking the store instead makes the durable half of the pair sufficient on its own.
     *
     * Safe to call repeatedly: `DeepLinkRouter` records what it has routed and ignores a repeat,
     * and the handler clears the entry once it is dealt with.
     */
    async resumePendingDeepLink(): Promise<boolean> {
        try {
            const router = await this.getRouter();
            return await router.processPending();
        } catch (err) {
            logError(`BVCApp: Error resuming pending deep link: ${err}`);
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
        // Stops the network side too, in the one order that leaves neither shared queue holding
        // audio for the session being torn down.
        await invoke("reset_asm");
    }
}
