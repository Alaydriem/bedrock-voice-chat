import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { warn } from '@tauri-apps/plugin-log';
import type { ChatAvailability } from '../../bindings/ChatAvailability';
import type { ChatWorld } from '../../bindings/ChatWorld';
import type { BedrockChatPayload, ChatLine } from './ChatLine';
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
    private poll: ReturnType<typeof setInterval> | null = null;

    /** The world the player is standing in right now, from the position pulse. */
    private liveWorld: string | null = null;
    private worlds: ChatWorld[] = [];
    private serverMode = false;

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
            if (here?.available) {
                return { kind: 'in-game', world: here };
            }
            if (here) {
                return { kind: 'unavailable', reason: `${here.world_name} is not running chat` };
            }
            // Standing somewhere the list does not know — a first join, before any history
            // row exists. Falling through to "some other world is usable" would name a world
            // the player is not in: the server would reject the send, but only after the
            // composer had spent the whole time claiming it would land there.
            return { kind: 'unavailable', reason: 'This world is not relaying chat' };
        }

        const usable = worlds.filter((w) => w.available);
        if (usable.length === 0) {
            return { kind: 'unavailable', reason: 'No world is running chat' };
        }
        if (usable.length === 1) {
            return { kind: 'only', world: usable[0] };
        }
        return { kind: 'choose', world: usable[0], options: usable };
    }

    /** No-net Bedrock: the local proxy is both the source and the destination. */
    async startLocal(): Promise<void> {
        await this.stop();
        this.unlisten = await listen<BedrockChatPayload>('bedrock-chat', (event) => {
            this.add(event.payload, false);
        });

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
        this.targetStore.set({
            kind: 'choose',
            world,
            options: this.worlds.filter((w) => w.available),
        });
        this.rejectionStore.set(null);
    }

    async stop(): Promise<void> {
        if (this.unlisten) {
            this.unlisten();
            this.unlisten = null;
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
            // The world list is itself the net-mode signal: a world only appears with a chat
            // channel registered for it. Asking the local proxy first would answer a different
            // question, and on a net-only server it would answer "no" while chat works.
            this.worlds = await invoke<ChatWorld[]>('chat_worlds').catch(() => []);
            this.serverMode = this.worlds.some((w) => w.available);

            if (this.serverMode) {
                this.applyTarget();
                return;
            }

            // No mod is relaying anywhere, so the only remaining source is this client's own
            // proxy — which is what the local check answers.
            const status = await invoke<ChatAvailability>('chat_availability');
            if (status.available) {
                this.targetStore.set({ kind: 'local' });
                return;
            }

            this.targetStore.set({
                kind: 'unavailable',
                reason: status.reason ?? 'Chat is unavailable',
            });
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

        const current = get(this.targetStore);
        const chosenStillValid =
            current.kind === 'choose' &&
            !this.liveWorld &&
            this.worlds.some((w) => w.world_uuid === current.world.world_uuid && w.available);

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
        const target = get(this.targetStore);
        if (target.kind === 'unavailable') {
            return;
        }

        this.rejectionStore.set(null);

        // No optimistic render. The realm echoes the line back and that echo is the
        // confirmation, so there is nothing to reconcile and no duplicate to suppress.
        if (target.kind !== 'local') {
            try {
                await invoke('chat_send', {
                    worldUuid: target.world.world_uuid,
                    text: trimmed,
                });
            } catch (e) {
                await warn(`chat send failed: ${e}`);
                this.rejectionStore.set({
                    kind: 'failed',
                    reason: String(e),
                    text: trimmed,
                });
            }
            return;
        }

        if (target.kind === 'local') {
            try {
                await invoke('bedrock_send_chat', { text: trimmed });
            } catch (e) {
                // Surfaced, not just logged. A composer that accepts a line and drops it
                // teaches the sender nothing, and they will assume it landed.
                await warn(`chat send failed: ${e}`);
                this.rejectionStore.set({
                    kind: 'failed',
                    reason: String(e),
                    text: trimmed,
                });
            }
        }
    }

    cleanup(): void {
        void this.stop();
    }

    private add(payload: BedrockChatPayload, fromApp: boolean): void {
        const line: ChatLine = {
            author: payload.author,
            text: payload.text,
            system: payload.system,
            fromApp,
            mention: this.isMention(payload),
            timestamp: ChatManager.stamp(),
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
