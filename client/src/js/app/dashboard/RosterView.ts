import type { NearbyPlayer } from './NearbyPlayer';

export interface RosterSplit {
    /** Players worth a whole card. */
    cards: readonly NearbyPlayer[];
    /** Everybody else, as an avatar. */
    chips: readonly NearbyPlayer[];
}

/**
 * How many of a section get a card, and how many get an avatar.
 *
 * Below the threshold everybody is a card, which is the design as it has always been. Above
 * it a card each stops fitting, and the room wants a different shape: a handful of cards over
 * a grid of faces.
 */
export class RosterView {
    /** Above this many in one section, a card each stops fitting. */
    static readonly DENSE_AT = 16;

    /**
     * Cards a dense section keeps.
     *
     * The nearest six, not the six who are talking. Promoting on speech changes the set every
     * couple of seconds, which reflows every avatar below it and needs a demotion hold to be
     * bearable — and an avatar whose ring is pulsing already answers "who is talking" without
     * anything moving.
     */
    static readonly CARD_LIMIT = 6;

    /** Assumes `players` is already nearest-first, which `NearbyManager` guarantees. */
    static split(players: readonly NearbyPlayer[]): RosterSplit {
        if (players.length < RosterView.DENSE_AT) {
            return { cards: players, chips: [] };
        }
        return {
            cards: players.slice(0, RosterView.CARD_LIMIT),
            chips: players.slice(RosterView.CARD_LIMIT),
        };
    }

    /** What the top bar says about the room. */
    static headline(inEarshot: number, approaching: number): string {
        if (inEarshot > 0) {
            return `${inEarshot} IN EARSHOT`;
        }
        if (approaching > 0) {
            return `${approaching} NEARBY`;
        }
        return 'NOBODY IN EARSHOT';
    }
}
