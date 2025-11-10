import App from './app.js';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { info, error as logError } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { DeepLinkRouter } from './deepLinkRouter.ts';
import type { DeepLink } from '../bindings/DeepLink';

export default class BVCApp extends App {
    private deepLinkRouter: DeepLinkRouter | null = null;
    private deepLinkUnlisten: UnlistenFn | null = null;
    private initialized = false;
    private storeInstance: Store | null = null;

    constructor() {
        super();
        this.setupDeepLinkListener();
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
     * Synchronously register the Tauri event listener
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
    }
}
