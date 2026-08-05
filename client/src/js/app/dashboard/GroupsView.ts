import { type Readable, type Writable, derived, writable } from 'svelte/store';
import type { Channel } from '../../bindings/Channel';
import GameNameUtils from '../utils/GameNameUtils';
import type { GroupMember, GroupRowView } from './GroupRowView';

/**
 * The groups pane's rows.
 *
 * Deliberately without a level on a group you have not joined. The server routes a channel's
 * audio only to its members, so this client receives nothing at all for a channel it is not
 * in — a meter there would be an invention. What can be said honestly is who is in it, which
 * of them you can currently hear, and when somebody last came or went.
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
     * @param audible Names this client can currently hear, in any name form.
     * @param self This client's own name, in any name form, for the ownership test.
     */
    rows(
        channels: readonly Channel[],
        joinedId: string | null,
        audible: ReadonlySet<string>,
        self = '',
    ): Readable<readonly GroupRowView[]> {
        return derived([this.activityStore, this.nowStore], ([$activity, $now]) =>
            channels.map((channel) => {
                const activeAt = $activity[channel.id] ?? null;
                return {
                    id: channel.id,
                    name: channel.name,
                    members: GroupsView.members(channel, audible),
                    joined: channel.id === joinedId,
                    // Compared through `namesMatch` because the creator is stored in whatever form
                    // the certificate's CN carried and this client knows itself by its gamertag.
                    owned: self !== '' && GameNameUtils.namesMatch(channel.creator, self),
                    activeAt,
                    stirring: activeAt !== null && $now - activeAt < GroupsView.STIR_MS,
                };
            }),
        );
    }

    private static members(channel: Channel, audible: ReadonlySet<string>): GroupMember[] {
        const heard = new Set([...audible].map((name) => GameNameUtils.stripPrefix(name)));
        return channel.players.map((player) => {
            const gamertag = GameNameUtils.stripPrefix(player);
            return {
                // The CN form where membership already carries it, so the glyph matches the one
                // on that player's card rather than being derived from a different string.
                name: player.includes(':') ? player : `minecraft:${player}`,
                gamertag,
                audible: heard.has(gamertag),
            };
        });
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
