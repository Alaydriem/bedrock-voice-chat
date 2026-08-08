/** One rendered line of server chat. */
export interface ChatLine {
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
}

/** The shape the Rust proxy emits on the `bedrock-chat` Tauri event. */
export interface BedrockChatPayload {
    author: string | null;
    text: string;
    system: boolean;
}
