import { Store } from '@tauri-apps/plugin-store';
import { info, error as logError } from '@tauri-apps/plugin-log';
import { AuthCallbackHandler } from './deepLinkHandlers/authCallbackHandler.ts';

/**
 * Outcome of handling a deep link.
 *
 * - `handled`  the link was fully processed; the pending entry can be cleared.
 * - `deferred` the handler navigated elsewhere (e.g. to /login) and needs the
 *              pending entry kept so the destination page can pick it up.
 */
export type DeepLinkOutcome = 'handled' | 'deferred';

interface DeepLinkHandler {
    canHandle(url: string): boolean;
    handle(url: string): Promise<DeepLinkOutcome>;
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
                    const outcome = await handler.handle(url);
                    if (outcome !== 'deferred') {
                        await this.clearPending();
                    }
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
