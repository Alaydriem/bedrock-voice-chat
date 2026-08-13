/**
 * Whether a line the app sent is known to have landed.
 *
 * `pending` and `failed` both render quietly: the sender must always see what they typed, and
 * a line nothing has confirmed must not look like one that arrived in the world. A line that
 * came *from* the world is `confirmed` on arrival — it is already there.
 */
export type ChatDelivery = 'pending' | 'confirmed' | 'failed';

/** One rendered line of server chat. */
export interface ChatLine {
    /** Distinguishes one pending send from another with the same text. */
    id: number;
    /** Absent for a server-authored line. */
    author: string | null;
    text: string;
    /** The server talking, not a person. */
    system: boolean;
    /** Sent from the app rather than typed in game. */
    fromApp: boolean;
    /** Addressed to the local player. */
    mention: boolean;
    /** HH:MM, stamped on arrival. */
    timestamp: string;
    delivery: ChatDelivery;
}

/** The shape the Rust proxy emits on the `bedrock-chat` Tauri event. */
export interface BedrockChatPayload {
    author: string | null;
    text: string;
    system: boolean;
}
