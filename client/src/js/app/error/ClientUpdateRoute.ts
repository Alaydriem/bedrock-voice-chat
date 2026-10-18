import { invoke } from "@tauri-apps/api/core";
import { info } from "@charlesportwoodii/tauri-plugin-curia";
import FaultCatalog from "./FaultCatalog";

/**
 * Turns "this client is too old" into "install the update" when this build can.
 *
 * The server list makes the same offer from its preflight, but a device with one saved server
 * is forwarded past that list and only ever learns it is too old from the connect. Without
 * this, VER01 is a screen whose only action reconnects into the same refusal.
 */
export class ClientUpdateRoute {
    private readonly checkForUpdates: () => Promise<string | null>;

    constructor(checkForUpdates?: () => Promise<string | null>) {
        this.checkForUpdates =
            checkForUpdates ?? (() => invoke<string | null>("check_for_updates"));
    }

    /** Where to go instead of `code`, or null to stay on it. */
    async replacementFor(code: string | null): Promise<string | null> {
        if (code !== FaultCatalog.CLIENT_TOO_OLD) return null;

        try {
            const version = await this.checkForUpdates();
            return version ? FaultCatalog.updateHref(version) : null;
        } catch (e) {
            info(`No updater available: ${e}`);
            return null;
        }
    }
}
