const GAME_PREFIXES = ['minecraft:', 'hytale:'];

export default class GameNameUtils {
    static stripPrefix(name: string): string {
        for (const prefix of GAME_PREFIXES) {
            if (name.startsWith(prefix)) {
                return name.slice(prefix.length);
            }
        }
        return name;
    }

    static namesMatch(a: string, b: string): boolean {
        return GameNameUtils.stripPrefix(a) === GameNameUtils.stripPrefix(b);
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
