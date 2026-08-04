import type ImageCache from '../components/imageCache';
import type { ServerListStore } from '../services/ServerListStore';
import type { PreflightObserver } from './preflight/PreflightRunner';
import type { PreflightOutcome } from './preflight/PreflightOutcome';

/** One server's four checks. Injected so a test never opens a socket. */
export type PreflightFactory = (
    server: string,
    observer: PreflightObserver,
) => Promise<PreflightOutcome>;

export interface ServerRosterDeps {
    readonly serverList: ServerListStore;
    readonly preflight: PreflightFactory;
    /** Operator art. Either asset can be absent, so neither can be relied on. */
    readonly imageCache: ImageCache;
    /** Clears one server's saved credentials. Injected so a test never touches a keyring. */
    readonly forgetCredentials: (server: string) => Promise<void>;
    /** The app version waiting to be installed, or null. Rejects where there is no updater. */
    readonly checkForUpdates: () => Promise<string | null>;
}
