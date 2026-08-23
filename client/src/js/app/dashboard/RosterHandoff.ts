import { Handoff, type Point } from '$radial/core/controllers/Handoff';
import type { NearbyPlayer } from './NearbyPlayer';

/** Where a player's bar sits on the ring, or null while the ring has not painted yet. */
export type RingPointResolver = (player: NearbyPlayer) => Point | null;

/**
 * The mark leaving the ring, and coming back.
 *
 * When somebody walks into earshot their bar flies off the ring and lands as their card's
 * avatar; when they leave it flies back. Nobody has to be told that the circle and the list are
 * the same information — they watch it happen once and then they know. `Handoff` has always been
 * able to draw the flight; nothing ever asked it to, which is why the roster simply appeared.
 *
 * Only the transitions fly — empty to occupied, and occupied to empty. Position snapshots arrive
 * at 2 Hz and every one of them re-renders the roster, so anything keyed on "the roster changed"
 * rather than on "the roster appeared" would launch a flyer for every card twice a second.
 *
 * The two directions need opposite moments, which is why this holds state rather than being a
 * function. An arrival can be measured after the fact: the card exists, so its avatar has a
 * position to fly to. A departure cannot — by the time anything knows the roster is empty, Svelte
 * has already removed the cards, and a detached element measures as 0x0 at the origin, which
 * would fling every flyer out of the top-left corner of the screen.
 */
export class RosterHandoff {
    /** Long enough to cover the flight, with a margin for a frame that ran late. */
    private static readonly SETTLE_MS = Handoff.FLIGHT_MS + 60;

    private readonly ringPoint: RingPointResolver;

    /** Whether the roster was on screen as of the last settle. */
    private showing = false;

    /**
     * The last roster that was on screen.
     *
     * Kept because a departure has to fly the people who were *there*, and the update that
     * triggers it carries the list that replaced them — which is empty.
     */
    private lastShown: readonly NearbyPlayer[] = [];

    /** Departing cards and where they were, read before the DOM lost them. */
    private departures: { player: NearbyPlayer; from: Point }[] = [];

    constructor(ringPoint: RingPointResolver) {
        this.ringPoint = ringPoint;
    }

    /**
     * Read the outgoing cards' positions, before the DOM is updated.
     *
     * Called with the roster as it still stands and with what is about to replace it. Only an
     * emptying roster is captured: a roster that is merely changing keeps its cards, and flying
     * them somewhere would be describing a departure that is not happening.
     */
    capture(roster: HTMLElement | null, willShow: boolean): void {
        this.departures = [];
        if (!roster || !this.showing || willShow) return;

        for (const player of this.lastShown) {
            const avatar = RosterHandoff.avatarOf(roster, player.name);
            if (avatar) this.departures.push({ player, from: Handoff.centreOf(avatar) });
        }
    }

    /**
     * Fly whatever the update implies, and report which cards are still in flight.
     *
     * The returned names are the ones whose cards must stay hidden until their flyer lands.
     * Hiding them is what makes an arrival read as the mark becoming the card rather than as a
     * card appearing while an unrelated block flies past it.
     *
     * Returns null when this update is not a transition, meaning "leave the held-back set alone".
     * An empty set would be wrong there: the update after an arrival lands half a second later,
     * which is sooner than the flight ends, and it would release every card mid-flight so the
     * flyer arrived at one already sitting in place.
     *
     * @param onLanded Called with a name once its flyer arrives, and with every outstanding name
     *   when the backstop fires. Reduced motion resolves a flight immediately, so a card must
     *   never be left invisible because an animation did not happen.
     */
    settle(
        roster: HTMLElement | null,
        players: readonly NearbyPlayer[],
        onLanded: (name: string) => void,
    ): ReadonlySet<string> | null {
        const wasShowing = this.showing;
        this.showing = players.length > 0;
        if (this.showing) this.lastShown = players;

        for (const { player, from } of this.departures) {
            const to = this.ringPoint(player);
            if (to) void Handoff.fly(from, to, player.hue);
        }
        this.departures = [];

        if (!this.showing) return new Set();
        if (wasShowing || !roster) return null;

        const flying = new Set<string>();
        for (const player of players) {
            const avatar = RosterHandoff.avatarOf(roster, player.name);
            const from = this.ringPoint(player);
            if (!avatar || !from) continue;
            flying.add(player.name);
            void Handoff.fly(from, Handoff.centreOf(avatar), player.hue).then(() =>
                onLanded(player.name),
            );
        }

        if (flying.size === 0) return null;

        setTimeout(() => {
            for (const name of flying) onLanded(name);
        }, RosterHandoff.SETTLE_MS);

        return flying;
    }

    /**
     * A card's avatar, found by the name stamped on the card.
     *
     * Queried rather than handed in, because the cards belong to a component tree this does not
     * own and their elements only exist between one render and the next.
     */
    private static avatarOf(roster: HTMLElement, name: string): Element | null {
        const card = roster.querySelector(`[data-card="${CSS.escape(name)}"]`);
        return card?.querySelector('.rad-player__avatar') ?? null;
    }
}
