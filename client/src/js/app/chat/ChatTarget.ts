import type { ChatWorld } from '../../bindings/ChatWorld';

/**
 * Where a composed message goes.
 *
 * Only `choose` carries any UI. Standing in a world settles the target, and offering a choice
 * there can only put someone's message in front of the wrong people.
 *
 * `local` is the no-net path: the proxy session is the world, so there is nothing to name.
 */
export type ChatTarget =
    | { kind: 'in-game'; world: ChatWorld }
    | { kind: 'only'; world: ChatWorld }
    | { kind: 'choose'; world: ChatWorld; options: ChatWorld[] }
    | { kind: 'local' }
    | { kind: 'unavailable'; reason: string };

/**
 * A send that did not land, and why.
 *
 * `moved` is the server refusing because the player changed worlds while the app still held
 * the old one. `failed` is everything else — no proxy session, a full queue, a dead link.
 * Either way the message went nowhere, which has to be said rather than logged.
 */
export type ChatRejectionState =
    | { kind: 'moved'; from: string; text: string }
    | { kind: 'failed'; reason: string; text: string };
