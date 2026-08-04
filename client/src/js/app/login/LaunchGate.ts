import { Store } from '@tauri-apps/plugin-store';
import { error as logError } from '@tauri-apps/plugin-log';
import { ServerListStore } from '../services/ServerListStore';

/**
 * Whether a launch owes the user an explanation before a credential prompt.
 *
 * The introduction is shown once. It is marked seen when someone leaves it — finishing the
 * last step or skipping — so quitting part-way through brings it back, and reaching the end
 * retires it for good. Signing out of every server does not: the explanation has been read,
 * and reading it again is not part of signing back in.
 *
 * An existing server list also counts as seen. Anyone who has signed in has been through
 * this, and an install that predates the flag should not be handed the introduction on its
 * next launch.
 *
 * It stays reachable from the sign-in screen's "What is this?" for anyone who wants it.
 */
export default class LaunchGate {
    static readonly SEEN_KEY = 'onboarding_seen';

    private readonly serverListStore: ServerListStore;

    constructor(serverListStore: ServerListStore = new ServerListStore()) {
        this.serverListStore = serverListStore;
    }

    static resolveEntry(seen: boolean, params: URLSearchParams): string {
        // Arriving with a server in hand is never a first-run situation: the
        // dashboard and the server list both land here that way.
        if (params.has('addserver') || params.has('reauth') || params.has('server')) {
            return 'login';
        }
        return seen ? 'login' : 'intro';
    }

    /** Whether the introduction has been read, or been made redundant by signing in. */
    async hasSeenOnboarding(): Promise<boolean> {
        if (await this.isMarkedSeen()) return true;
        return this.hasServers();
    }

    /**
     * Record that the introduction has been read.
     *
     * Failing to persist this costs one repeat of the introduction, so it is logged and
     * swallowed rather than allowed to interrupt the flow it is being called from.
     */
    async markSeen(): Promise<void> {
        try {
            const store = await this.store();
            await store.set(LaunchGate.SEEN_KEY, true);
            await store.save();
        } catch (e) {
            logError(`LaunchGate: could not record that onboarding was seen: ${e}`);
        }
    }

    async hasServers(): Promise<boolean> {
        const servers = await this.serverListStore.getServerList();
        return servers.length > 0;
    }

    private async isMarkedSeen(): Promise<boolean> {
        try {
            const store = await this.store();
            return (await store.get<boolean>(LaunchGate.SEEN_KEY)) === true;
        } catch (e) {
            logError(`LaunchGate: could not read the onboarding marker: ${e}`);
            return false;
        }
    }

    private store(): Promise<Store> {
        return Store.load('store.json', { autoSave: false, defaults: {} });
    }
}
