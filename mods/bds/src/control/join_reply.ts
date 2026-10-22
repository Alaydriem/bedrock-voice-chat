import type { ControlSendResult } from './sender/control_sender';

/// What a player reads after asking to join a group from the game.
///
/// A protected group is joined in the player's BVC app: a member there compares a code with
/// them and admits them. The game never shows the code and never says "Joined" for one.
export class JoinReply {
  static forResult(
    result: ControlSendResult,
    name: string,
    isProtected: boolean,
  ): string {
    if (result.outcome === 'no_session') {
      return `§c[BVC] Open the BVC app to join ${name}.`;
    }
    // No-net mode has no reply; its request goes to the player's own app all the same.
    if (result.outcome === 'forwarded' || (isProtected && result.ok)) {
      return `§e[BVC] Your code for ${name} is in the BVC app.`;
    }
    if (!result.ok) {
      return '§c[BVC] Group join failed; try again';
    }
    return `§a[BVC] Joined §f${name}`;
  }
}
