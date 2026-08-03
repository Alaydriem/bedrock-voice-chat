import { ServerListStore } from '../services/ServerListStore';

/**
 * Whether a launch owes the user an explanation before a credential prompt.
 *
 * Derived from the server list rather than a stored marker. Signing in populates
 * `server_list`, so that list *is* the record of having got in: there is nothing to
 * clear on success and nothing that can drift out of step.
 *
 * It also covers the case a flag was originally proposed for — quitting part-way
 * through the introduction leaves the list still empty, so the introduction returns
 * without anything having had to persist correctly.
 *
 * Signing out of every server replays the introduction. That is the honest reading
 * of "unauthenticated users get onboarded", not a regression.
 */
export default class LaunchGate {
    private readonly serverListStore: ServerListStore;

    constructor(serverListStore: ServerListStore = new ServerListStore()) {
        this.serverListStore = serverListStore;
    }

    static resolveEntry(hasServers: boolean, params: URLSearchParams): string {
        // Arriving with a server in hand is never a first-run situation: the
        // dashboard and the server list both land here that way.
        if (params.has('addserver') || params.has('reauth') || params.has('server')) {
            return 'login';
        }
        return hasServers ? 'login' : 'intro';
    }

    async hasServers(): Promise<boolean> {
        const servers = await this.serverListStore.getServerList();
        return servers.length > 0;
    }
}
