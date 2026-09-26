import { get, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { error } from '@charlesportwoodii/tauri-plugin-curia';
import type { VoiceRoster } from '../../bindings/VoiceRoster';
import type { VoiceMember } from '../../bindings/VoiceMember';
import type { NearbyPlayer } from '../dashboard/NearbyPlayer';
import type { PlayerManager } from './PlayerManager';
import { Coalescer } from '../utils/Coalescer';

/**
 * Publishes who this client can hear to the `/events` WebSocket, for the OBS overlay.
 *
 * Proximity comes from the position feed and group membership from PlayerManager, because they
 * are audible for different reasons and only one of them is a range test. Group audio carries
 * at any distance, so a member across the map is audible and absent from the feed's earshot
 * set; a neighbour is audible because they are close, and the feed is the only thing that knows
 * where anyone is.
 *
 * PlayerManager's own proximity source is deliberately not used. It is fed by presence events,
 * which fire on a player's first datagram, so a client that connected after them never hears
 * about them — the roster reads empty while those people are plainly audible.
 */
export class VoiceRosterPublisher {
    /**
     * How often a roster change is allowed out of the webview.
     *
     * The position feed arrives at 2 Hz and a group join adds several members in one tick;
     * neither should be an IPC call per member. Short enough that a person appearing is not
     * perceptibly late on a stream.
     */
    private static readonly PUBLISH_GAP_MS = 150;

    private readonly playerManager: PlayerManager;
    private readonly inEarshot: Readable<readonly NearbyPlayer[]>;
    private readonly unsubscribes: Array<() => void> = [];
    private readonly writes = new Coalescer(VoiceRosterPublisher.PUBLISH_GAP_MS, () =>
        this.publish(),
    );

    constructor(playerManager: PlayerManager, inEarshot: Readable<readonly NearbyPlayer[]>) {
        this.playerManager = playerManager;
        this.inEarshot = inEarshot;
    }

    /**
     * The roster as it stands, without publishing it.
     *
     * Sorted by name. Activity order would reorder tiles while people speak, which on a stream
     * reads as the overlay malfunctioning.
     */
    project(): VoiceRoster {
        const own = this.playerManager.getCurrentUser();
        const members = new Map<string, VoiceMember>();

        for (const player of get(this.inEarshot)) {
            // `game` presence is somebody in the world with no voice connection. Nothing they
            // say arrives and nothing said to them lands, so they are not on this roster.
            if (player.presence !== 'voice') continue;
            if (player.name === own) continue;
            members.set(player.name, {
                name: player.name,
                sources: ['Proximity'],
                gamerpic: this.playerManager.get(player.name)?.gamerpic ?? null,
            });
        }

        for (const player of get(this.playerManager.activePlayers)) {
            if (!player.sources.has('Group')) continue;
            const already = members.get(player.name);
            if (already) {
                already.sources = ['Group', 'Proximity'];
                continue;
            }
            members.set(player.name, {
                name: player.name,
                sources: ['Group'],
                gamerpic: player.gamerpic ?? null,
            });
        }

        return {
            own,
            members: [...members.values()].sort((a, b) => a.name.localeCompare(b.name)),
        };
    }

    start(): void {
        this.cleanup();
        this.unsubscribes.push(this.inEarshot.subscribe(() => this.writes.request()));
        this.unsubscribes.push(
            this.playerManager.activePlayers.subscribe(() => this.writes.request()),
        );
    }

    cleanup(): void {
        this.writes.cancel();
        while (this.unsubscribes.length) {
            this.unsubscribes.pop()?.();
        }
    }

    private async publish(): Promise<void> {
        try {
            await invoke('publish_voice_roster', { roster: this.project() });
        } catch (err) {
            // The overlay keeps the roster it already has. A failure here is not worth
            // interrupting anything the user is doing.
            error(`VoiceRosterPublisher: could not publish the roster: ${err}`);
        }
    }
}
