import { BedrockManager } from '../managers/bedrock/BedrockManager';
import { BedrockCapabilityManager } from '../managers/bedrock/BedrockCapabilityManager';

/**
 * The session's one `BedrockManager`, built on first use.
 *
 * Lazy, because constructing the manager restores the Microsoft session and reads
 * `/api/config`; a session that never opens the Connect pane must not pay for either.
 * Long-lived, because the connection log lives on it — the settings screen used to own
 * the manager, so closing settings threw the log away and reopening showed an empty pane
 * while the proxy was still running.
 *
 * Owns the capability manager it constructs, which nothing else does: that one attaches a
 * window focus listener and schedules its own retries, and an undestroyed instance keeps
 * refreshing behind a screen that has gone.
 */
export class BedrockManagerHolder {
    private manager: BedrockManager | null = null;
    private capability: BedrockCapabilityManager | null = null;

    get(): BedrockManager {
        if (!this.manager) {
            this.capability = new BedrockCapabilityManager();
            this.manager = new BedrockManager(this.capability);
            // Loads both managers from the store and restores the Microsoft session.
            // Guards itself against running twice.
            void this.manager.initialize();
        }
        return this.manager;
    }

    destroy(): void {
        this.manager?.destroy();
        this.manager = null;
        this.capability?.destroy();
        this.capability = null;
    }
}
