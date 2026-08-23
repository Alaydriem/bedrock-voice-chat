import type { PresenceKind } from '../../bindings/PresenceKind';

/** One player the feed says is near you. */
export interface NearbyPlayer {
    /** The CN form as received, `game:gamertag`. The identity everything else keys on. */
    name: string;
    /** Bare gamertag: what `PlayerManager` and the persisted gain store key on. */
    gamertag: string;
    game: string;
    /** Their colour, derived from the lowercased CN so it agrees with their glyph. */
    hue: string;
    presence: PresenceKind;
    /** Metres. */
    distance: number;
    /** Radians, relative to where the listener is facing. */
    bearing: number;
    /** Blocks above or below, so the UI can tell a rooftop from a doorway. */
    elevation: number;
    /** True while they are inside the server's voice range, and so on the roster. */
    inEarshot: boolean;
}
