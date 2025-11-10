import { Store } from '@tauri-apps/plugin-store';
import { info, error as logError } from '@tauri-apps/plugin-log';
import { AuthCallbackHandler } from './deepLinkHandlers/authCallbackHandler.ts';

interface DeepLinkHandler {
    canHandle(url: string): boolean;
    handle(url: string): Promise<void>;
}

export class DeepLinkRouter {
    private handlers: DeepLinkHandler[] = [];
    private readonly PENDING_KEY = "pending_deep_link";
    private store: Store;

    constructor(store: Store) {
        this.store = store;
        this.handlers.push(new AuthCallbackHandler(store));
    }

    /**
     * Route a deep link URL to the appropriate handler
     */
    async route(url: string): Promise<void> {
        info(`DeepLinkRouter: Routing URL: ${url}`);

        for (const handler of this.handlers) {
            if (handler.canHandle(url)) {
                info(`DeepLinkRouter: Handler found for URL`);
                try {
                    await handler.handle(url);
                    await this.clearPending();
                    return;
                } catch (err) {
                    logError(`DeepLinkRouter: Handler failed: ${err}`);
                    await this.clearPending();
                    throw err;
                }
            }
        }

        await this.clearPending();
        throw new Error(`No handler found for URL: ${url}`);
    }

    /**
     * Process any pending deep links from storage
     */
    async processPending(): Promise<boolean> {
        const url = await this.store.get<string>(this.PENDING_KEY);
        if (!url) {
            return false;
        }

        info(`DeepLinkRouter: Found pending deep link: ${url}`);
        await this.route(url);
        return true;
    }

    /**
     * Clear pending deep link from storage
     */
    async clearPending(): Promise<void> {
        await this.store.delete(this.PENDING_KEY);
        await this.store.save();
    }
}
