import { type Readable, type Writable, derived, writable } from 'svelte/store';
import { PlayerHue } from '$radial/core/sources/PlayerHue';
import type { Channel } from '../../bindings/Channel';
import GameNameUtils from '../utils/GameNameUtils';
import type { GroupMember, GroupRowView } from './GroupRowView';

/**
 * The groups pane's rows.
 *
 * Deliberately without a level on a group you have not joined. The server routes a channel's
 * audio only to its members, so this client receives nothing at all for a channel it is not
 * in — a meter there would be an invention. What can be said honestly is who is in it, and
 * when somebody last came or went.
 */
export class GroupsView {
    /** How long a join or leave keeps a row stirring. */
    static readonly STIR_MS = 6_000;

    private readonly activityStore: Writable<Record<string, number>>;
    private readonly nowStore: Writable<number>;
    private tick: ReturnType<typeof setInterval> | null = null;

    constructor() {
        this.activityStore = writable({});
        this.nowStore = writable(Date.now());
    }

    /**
     * Start the clock that ages the stir out.
     *
     * A row has to stop stirring on its own: the event that started it is the last one that
     * will arrive, so nothing else would ever re-render it.
     */
    start(): void {
        this.stop();
        this.tick = setInterval(() => this.nowStore.set(Date.now()), 1_000);
    }

    /** Record a join or leave. Channel events reach every client, including for groups you are not in. */
    stir(channelId: string): void {
        this.activityStore.update((current) => ({ ...current, [channelId]: Date.now() }));
    }

    /**
     * @param channels Every channel on the server.
     * @param joinedId The channel this client is in, if any.
     * @param self This client's canonical identity, for the ownership test.
     */
    rows(
        channels: readonly Channel[],
        joinedId: string | null,
        self = '',
    ): Readable<readonly GroupRowView[]> {
        return derived([this.activityStore, this.nowStore], ([$activity, $now]) =>
            channels.map((channel) => {
                const activeAt = $activity[channel.id] ?? null;
                return {
                    id: channel.id,
                    name: channel.name,
                    members: GroupsView.members(channel),
                    joined: channel.id === joinedId,
                    // Exact. The creator is the certificate's Common Name and so is `self`, and a
                    // comparison that tolerated the bare form would hand the close button for
                    // `hytale:Bob`'s group to `minecraft:Bob`.
                    owned: self !== '' && channel.creator === self,
                    activeAt,
                    stirring: activeAt !== null && $now - activeAt < GroupsView.STIR_MS,
                };
            }),
        );
    }

    private static members(channel: Channel): GroupMember[] {
        return channel.players.map((player) => {
            // Membership already carries the canonical form; composing it again costs nothing
            // and is what keeps this row's hue derived from the same string as that player's
            // card rather than from a different one.
            const name = GameNameUtils.canonical(player);
            const gamertag = GameNameUtils.stripPrefix(name);
            return {
                name,
                gamertag,
                hue: PlayerHue.of(name.toLowerCase()),
                initials: GroupsView.initials(gamertag),
            };
        });
    }

    /**
     * The letters a face carries.
     *
     * Word initials where there are words to take them from: Xbox gamertags may contain
     * spaces, and "SG" tells two people called "Some Gamer" and "Sombra" apart where the
     * first two characters of each do not.
     */
    private static initials(gamertag: string): string {
        const words = gamertag.trim().split(/\s+/).filter(Boolean);
        if (words.length === 0) return '?';
        if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
        return (words[0][0] + words[1][0]).toUpperCase();
    }

    /** "active 2 min ago", or nothing when no join or leave has been seen. */
    static since(activeAt: number | null, now: number): string {
        if (activeAt === null) return '';
        const seconds = Math.max(0, Math.round((now - activeAt) / 1000));
        if (seconds < 60) return 'active just now';
        const minutes = Math.round(seconds / 60);
        if (minutes < 60) return `active ${minutes} min ago`;
        const hours = Math.round(minutes / 60);
        return `active ${hours} h ago`;
    }

    stop(): void {
        if (this.tick) {
            clearInterval(this.tick);
            this.tick = null;
        }
    }
}
