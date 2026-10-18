import { I18n } from "#lib/i18n/index.js";
/**
 * What "Copy message" copies.
 *
 * Written to be forwarded verbatim to someone who has not heard of BVC: it says what
 * is being asked for, how long it takes, and what to send back. A link on its own
 * gets ignored.
 */
export default class InviteMessage {
    /**
     * A getter, not a field, so it is read in the language loaded when the button is pressed.
     *
     * One paragraph per marker: a sentence split across markers cannot be translated, because
     * no other language breaks it at the same word.
     */
    static get TEXT(): string {
        return [
            I18n.t("Hey — I'd like to use proximity voice chat on our Minecraft world."),
            "",
            I18n.t(
                "It's a one-time setup on your side: run the BVC server on any machine you control, and add a small mod to the world. The guide walks through both:",
            ),
            "https://bedrockvoicechat.com/wiki",
            "",
            I18n.t(
                "Takes about fifteen minutes. Once it's running, send me the server address and everyone signs in with the account they already play on.",
            ),
        ].join("\n");
    }
}
