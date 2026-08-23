import type { Game } from "../../bindings/Game";

/**
 * Who the client is signed in as, on the server it is signed in to.
 *
 * Both halves, always: the server keys a player on `game` and `gamertag` together, and a
 * gamertag alone matches a different person in the other game.
 */
export interface ViewerIdentity {
    readonly gamertag: string;
    readonly game: Game;
}
