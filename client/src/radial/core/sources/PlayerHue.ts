import { Hash } from "../math/Hash";
import { MarkData } from "../mark/MarkData";

/**
 * A player's colour.
 *
 * Assigned from a column of the mark, keyed on the certificate CN form
 * `game:gamertag` — the same key channel membership uses, so a player is one
 * identity across the roster, the ring, the chat log and a recorded track.
 *
 * This supersedes the client's current 24-colour hash. Two consequences worth
 * knowing before it is adopted: existing players will change colour once, and the
 * palette is 23 wide, so a large server will repeat hues. Repetition is acceptable
 * because the hue is a recognition aid beside a name, never the identifier.
 */
export class PlayerHue {
  /** @param key `game:gamertag`, e.g. `minecraft:Alaydriem`. */
  static of(key: string): string {
    return MarkData.hueAt(PlayerHue.columnOf(key));
  }

  static columnOf(key: string): number {
    return Hash.fnv1a(key.toLowerCase()) % MarkData.COLS;
  }

  static forPlayer(game: string, gamertag: string): string {
    return PlayerHue.of(`${game}:${gamertag}`);
  }
}
