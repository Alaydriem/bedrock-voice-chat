const GAME_PREFIXES = ['minecraft:', 'hytale:'];

export default class GameNameUtils {
    /**
     * The canonical identity for a player: `game:gamertag`.
     *
     * The only place a canonical name is produced on the client. Every key, map index and
     * identity comparison goes through here, so a name that entered from the certificate, the
     * position feed or a packet all reduce to the same string.
     *
     * A name that already declares its game keeps it — re-prefixing with the caller's guess
     * would move a player from one game to another. An empty name is not a player and stays
     * empty, because `minecraft:` is a key that matches nobody and never expires.
     */
    static canonical(name: string, game: string = 'minecraft'): string {
        if (name.trim() === '') {
            return '';
        }
        for (const prefix of GAME_PREFIXES) {
            if (name.startsWith(prefix)) {
                return name;
            }
        }
        return `${game}:${name}`;
    }

    /**
     * The bare gamertag, for display only.
     *
     * Never use the result as a key, a map index, or either side of an identity comparison:
     * two games can carry the same gamertag, so the bare form merges two players into one.
     * Use `canonical` and `===` instead.
     */
    static stripPrefix(name: string): string {
        for (const prefix of GAME_PREFIXES) {
            if (name.startsWith(prefix)) {
                return name.slice(prefix.length);
            }
        }
        return name;
    }

    static extractGame(name: string): string {
        for (const prefix of GAME_PREFIXES) {
            if (name.startsWith(prefix)) {
                return prefix.slice(0, -1);
            }
        }
        return 'minecraft';
    }
}
