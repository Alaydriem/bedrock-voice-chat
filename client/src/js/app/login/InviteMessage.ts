/**
 * What "Copy message" copies.
 *
 * Written to be forwarded verbatim to someone who has not heard of BVC: it says what
 * is being asked for, how long it takes, and what to send back. A link on its own
 * gets ignored.
 */
export default class InviteMessage {
    static readonly TEXT = [
        "Hey — I'd like to use proximity voice chat on our Minecraft world.",
        '',
        "It's a one-time setup on your side: run the BVC server on any machine you control,",
        'and add a small mod to the world. The guide walks through both:',
        'https://bedrockvoicechat.com/wiki',
        '',
        'Takes about fifteen minutes. Once it\'s running, send me the server address and',
        'everyone signs in with the account they already play on.',
    ].join('\n');
}
