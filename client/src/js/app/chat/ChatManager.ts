import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { warn } from '@tauri-apps/plugin-log';
import type { ChatTransport } from '../../bindings/ChatTransport';
import type { ChatWorld } from '../../bindings/ChatWorld';
import type { BedrockChatPayload, ChatDelivery, ChatLine } from './ChatLine';
import type { ChatRejectionState, ChatTarget } from './ChatTarget';

/**
 * Server chat, relayed live.
 *
 * History begins when you connect and ends when you leave. Nothing is stored — not on disk,
 * not in the Tauri store, not on the server. The only history that exists is this ring.
 */
export class ChatManager {
    /** Matches the radial reference. Enough to scroll back through a conversation, bounded so
     *  a long session cannot grow without limit. */
    static readonly LIMIT = 200;

    private linesStore: Writable<ChatLine[]>;
    private unreadStore: Writable<number>;
    private targetStore: Writable<ChatTarget>;
    private rejectionStore: Writable<ChatRejectionState | null>;

    public readonly lines: Readable<ChatLine[]>;
    public readonly unread: Readable<number>;
    public readonly target: Readable<ChatTarget>;
    public readonly rejection: Readable<ChatRejectionState | null>;
    public readonly canSend: Readable<boolean>;

    /**
     * Liveness is polled rather than pushed.
     *
     * It has several independent inputs — a proxy starting, a session ending, positions
     * arriving — and one missed event on any of them would strand the composer in the wrong
     * state permanently. A poll self-corrects; a dropped event does not. 1 Hz matches the
     * self-state poll the dashboard already runs.
     */
    static readonly AVAILABILITY_POLL_MS = 1000;

    private open = false;
    private unlisten: UnlistenFn | null = null;
    private unlistenWorld: UnlistenFn | null = null;
    private unlistenRejected: UnlistenFn | null = null;
    private poll: ReturnType<typeof setInterval> | null = null;

    /** The world the player is standing in right now, from the position pulse. */
    private liveWorld: string | null = null;
    private worlds: ChatWorld[] = [];
    private serverMode = false;

    /**
     * How long a sent line may claim to be in flight.
     *
     * Long enough to cover a slow round trip, short enough that nobody has walked away
     * believing a message landed. Public so a test can advance past it without duplicating
     * the number.
     */
    static readonly ANSWER_WINDOW_MS = 8_000;

    private lastLineId = 0;
    private readonly answerTimers = new Map<number, ReturnType<typeof setTimeout>>();

    constructor(private readonly selfName: string) {
        this.linesStore = writable([]);
        this.unreadStore = writable(0);
        this.targetStore = writable({ kind: 'unavailable', reason: 'Not connected' });
        this.rejectionStore = writable(null);

        this.lines = { subscribe: this.linesStore.subscribe };
        this.unread = { subscribe: this.unreadStore.subscribe };
        this.target = { subscribe: this.targetStore.subscribe };
        this.rejection = { subscribe: this.rejectionStore.subscribe };
        this.canSend = derived(this.targetStore, ($t) => $t.kind !== 'unavailable');
    }

    /**
     * Decides where a composed message goes.
     *
     * Addressability only. A world the server has told us about can be addressed; whether its
     * addon is answering right now is not consulted, because that reading flaps and the send
     * path reports a real failure when one happens. `available` is display, never a gate.
     *
     * Static and pure so the rule lives in one place and can be tested without a running app.
     * This is a correctness concern rather than a convenience: picking wrong delivers someone's
     * message into a room they are not in.
     */
    static resolveTarget(worlds: ChatWorld[], liveWorldUuid: string | null): ChatTarget {
        // In game the server already knows where the player is standing. No picker is offered,
        // because the only thing a choice can do here is put the message in front of the wrong
        // people.
        if (liveWorldUuid) {
            const here = worlds.find((w) => w.world_uuid === liveWorldUuid);
            if (here) {
                return { kind: 'in-game', world: here };
            }
            // Standing somewhere the list does not know — a first join, before any history
            // row exists. Falling through to another world would name one the player is not
            // in: the server would reject the send, but only after the composer had spent the
            // whole time claiming it would land there.
            return { kind: 'unavailable', reason: 'This world is not known to the server yet' };
        }

        if (worlds.length === 0) {
            return { kind: 'unavailable', reason: 'No world to send to yet' };
        }
        if (worlds.length === 1) {
            return { kind: 'only', world: worlds[0] };
        }
        return { kind: 'choose', world: worlds[0], options: worlds };
    }

    /** No-net Bedrock: the local proxy is both the source and the destination. */
    async startLocal(): Promise<void> {
        await this.stop();
        this.unlisten = await listen<BedrockChatPayload>('bedrock-chat', (event) => {
            this.acceptLine(event.payload);
        });

        // The server refusing a line this client sent. Its own event rather than a chat line:
        // it settles the pending line already on screen instead of adding one.
        this.unlistenRejected = await listen<{ reason: string; text: string }>(
            'bedrock-chat-rejected',
            (event) => this.handleRejection(event.payload),
        );

        // The player's live world, pulsed by the server on every position frame. Nothing else
        // surfaces it, and a mid-session transfer re-targets chat off the back of it.
        this.unlistenWorld = await listen<string | null>('chat-world', (event) => {
            const next = event.payload ?? null;
            if (next === this.liveWorld) return;
            this.liveWorld = next;
            this.applyTarget();
        });

        await this.refreshAvailability();
        this.poll = setInterval(() => {
            void this.refreshAvailability();
        }, ChatManager.AVAILABILITY_POLL_MS);
    }

    /** Off-game picker: the player chose which world to post to. */
    choose(world: ChatWorld): void {
        // Every known world stays on offer. Filtering by a registered chat channel would drop
        // a world from the picker the moment its addon blinked, mid-conversation.
        this.targetStore.set({
            kind: 'choose',
            world,
            options: this.worlds,
        });
        this.rejectionStore.set(null);
    }

    async stop(): Promise<void> {
        if (this.unlisten) {
            this.unlisten();
            this.unlisten = null;
        }
        if (this.unlistenRejected) {
            this.unlistenRejected();
            this.unlistenRejected = null;
        }
        if (this.unlistenWorld) {
            this.unlistenWorld();
            this.unlistenWorld = null;
        }
        if (this.poll !== null) {
            clearInterval(this.poll);
            this.poll = null;
        }
    }

    /**
     * Asks the backend whether typing will accomplish anything, and says so in the composer.
     *
     * A composer that looks live and silently drops what you type is worse than one that is
     * visibly shut, because the sender walks away believing the message landed.
     */
    private async refreshAvailability(): Promise<void> {
        try {
            // The declared addon mode is the signal, not the world list. A world only carries
            // `available` while its chat socket happens to be up, and treating that as the
            // net-mode answer routed a net world down the no-net path the moment BDS blinked.
            this.worlds = await invoke<ChatWorld[]>('chat_worlds').catch(() => []);
            const transport = await invoke<ChatTransport>('chat_transport');

            // The proxy owns chat only on a no-net world, and there it is both source and
            // destination — so there is nothing to name.
            if (transport === 'proxy_injection') {
                this.serverMode = false;
                this.targetStore.set({ kind: 'local' });
                return;
            }

            this.serverMode = true;
            this.applyTarget();
        } catch (e) {
            await warn(`chat availability check failed: ${e}`);
            this.targetStore.set({ kind: 'unavailable', reason: 'Chat is unavailable' });
        }
    }

    /**
     * Recomputes where a message would go.
     *
     * Keeps an explicit off-game choice: re-resolving on every poll would drag the picker back
     * to the default a second after somebody used it.
     */
    private applyTarget(): void {
        if (!this.serverMode) {
            this.targetStore.set({ kind: 'local' });
            return;
        }

        // An explicit choice survives its world's addon going quiet. Requiring a registered
        // channel here dragged the picker back to the default mid-conversation.
        const current = get(this.targetStore);
        const chosenStillValid =
            current.kind === 'choose' &&
            !this.liveWorld &&
            this.worlds.some((w) => w.world_uuid === current.world.world_uuid);

        if (chosenStillValid) {
            return;
        }

        this.targetStore.set(ChatManager.resolveTarget(this.worlds, this.liveWorld));
    }

    setOpen(open: boolean): void {
        this.open = open;
        if (open) {
            this.unreadStore.set(0);
        }
    }

    clearRejection(): void {
        this.rejectionStore.set(null);
    }

    async send(text: string): Promise<void> {
        const trimmed = text.trim();
        if (!trimmed) {
            return;
        }

        this.rejectionStore.set(null);

        // Rendered before anything is attempted, and never gated on a target. The sender must
        // see what they typed even when nothing can carry it — an unconfirmed line they can
        // read beats a line that was silently discarded because delivery was not provable.
        const id = this.appendPending(trimmed);
        const target = get(this.targetStore);

        try {
            if (target.kind === 'local') {
                await invoke('bedrock_send_chat', { text: trimmed });
            } else {
                // Undefined where no world is known. The server answers `NoWorld` rather than
                // this guessing on its behalf.
                const worldUuid = target.kind === 'unavailable' ? undefined : target.world.world_uuid;
                await invoke('chat_send', { worldUuid, text: trimmed });
            }
        } catch (e) {
            // Surfaced, not just logged. A composer that accepts a line and drops it teaches
            // the sender nothing, and they will assume it landed.
            await warn(`chat send failed: ${e}`);
            this.settle(id, 'failed');
            this.rejectionStore.set({ kind: 'failed', reason: String(e), text: trimmed });
            return;
        }

        // Queued, not delivered. Only an echo proves it arrived, so a line nothing answers
        // inside the window stops claiming to be in flight.
        const timer = setTimeout(() => {
            this.answerTimers.delete(id);
            this.settle(id, 'failed');
        }, ChatManager.ANSWER_WINDOW_MS);
        this.answerTimers.set(id, timer);
    }

    /**
     * A send the server refused.
     *
     * Settles the newest pending line carrying that text and raises the reason beside the
     * composer. Both matter: the notice explains it once, and the greyed line keeps saying so
     * after the notice is dismissed.
     */
    handleRejection(payload: { reason: string; text: string }): void {
        const id = this.newestPendingId(payload.text);
        if (id !== null) {
            this.settle(id, 'failed');
        }
        this.rejectionStore.set({
            kind: 'failed',
            reason: payload.reason,
            text: payload.text,
        });
    }

    /** The most recent unsettled send carrying this text, if the sender still has one. */
    private newestPendingId(text: string): number | null {
        const pending = get(this.linesStore).filter(
            (line) => line.delivery === 'pending' && line.text === text,
        );
        return pending.length === 0 ? null : pending[pending.length - 1].id;
    }

    /**
     * Takes a line reported by the world.
     *
     * Public because the Tauri listener and the tests are two callers of one behaviour, and a
     * listener that closed over private state could not be exercised without the whole app.
     */
    acceptLine(payload: BedrockChatPayload): void {
        if (this.promotePending(payload)) {
            return;
        }
        this.add(payload, false);
    }

    /** Appends the sender's own line as unconfirmed and returns its id. */
    private appendPending(text: string): number {
        const id = ++this.lastLineId;
        const line: ChatLine = {
            id,
            author: this.selfName,
            text,
            system: false,
            fromApp: true,
            mention: false,
            timestamp: ChatManager.stamp(),
            delivery: 'pending',
        };

        this.linesStore.update(($lines) => {
            const next = [...$lines, line];
            return next.length > ChatManager.LIMIT ? next.slice(-ChatManager.LIMIT) : next;
        });

        return id;
    }

    /**
     * Confirms the oldest pending line this echo matches, if any.
     *
     * Matched on the sender's own name and the exact text: another player repeating your words
     * is not evidence that your send arrived.
     */
    private promotePending(payload: BedrockChatPayload): boolean {
        if (payload.system || payload.author !== this.selfName) {
            return false;
        }

        let promoted = false;
        this.linesStore.update(($lines) =>
            $lines.map((line) => {
                if (promoted || line.delivery !== 'pending' || line.text !== payload.text) {
                    return line;
                }
                promoted = true;
                this.clearAnswerTimer(line.id);
                return { ...line, delivery: 'confirmed' as const };
            }),
        );

        return promoted;
    }

    private settle(id: number, delivery: ChatDelivery): void {
        this.clearAnswerTimer(id);
        this.linesStore.update(($lines) =>
            $lines.map((line) => (line.id === id ? { ...line, delivery } : line)),
        );
    }

    private clearAnswerTimer(id: number): void {
        const timer = this.answerTimers.get(id);
        if (timer) {
            clearTimeout(timer);
            this.answerTimers.delete(id);
        }
    }

    cleanup(): void {
        void this.stop();
    }

    private add(payload: BedrockChatPayload, fromApp: boolean): void {
        const line: ChatLine = {
            id: ++this.lastLineId,
            author: payload.author,
            text: payload.text,
            system: payload.system,
            fromApp,
            mention: this.isMention(payload),
            timestamp: ChatManager.stamp(),
            // Reported by the world, so it is already there.
            delivery: 'confirmed',
        };

        this.linesStore.update(($lines) => {
            const next = [...$lines, line];
            return next.length > ChatManager.LIMIT ? next.slice(-ChatManager.LIMIT) : next;
        });

        if (!this.open) {
            this.unreadStore.update((n) => n + 1);
        }
    }

    private isMention(payload: BedrockChatPayload): boolean {
        if (payload.system || !this.selfName) {
            return false;
        }
        // Your own line is not a mention of you, even when you type your own name.
        if (payload.author === this.selfName) {
            return false;
        }
        return payload.text.toLowerCase().includes(this.selfName.toLowerCase());
    }

    private static stamp(now = new Date()): string {
        const h = String(now.getHours()).padStart(2, '0');
        const m = String(now.getMinutes()).padStart(2, '0');
        return `${h}:${m}`;
    }
}
