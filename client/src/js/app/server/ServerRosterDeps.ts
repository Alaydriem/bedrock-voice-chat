import type { ServerHealthService } from '../services/ServerHealthService';
import type { ServerListStore } from '../services/ServerListStore';

export interface ServerRosterDeps {
    readonly health: ServerHealthService;
    readonly serverList: ServerListStore;
    /** Clears one server's saved credentials. Injected so a test never touches a keyring. */
    readonly forgetCredentials: (server: string) => Promise<void>;
    /** The app version waiting to be installed, or null. Rejects where there is no updater. */
    readonly checkForUpdates: () => Promise<string | null>;
}
