import { get } from 'svelte/store';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { mockInvoke } from '../../tauri';
import { ChatManager } from '../../../js/app/chat/ChatManager';
import type { ChatWorld } from '../../../js/bindings/ChatWorld';

function world(uuid: string, name: string, available = true): ChatWorld {
    return {
        world_uuid: uuid,
        world_name: name,
        last_seen: 0n,
        active: true,
        available,
        mode: 'Server',
    };
}

// A line the sender cannot see is the whole bug. Nothing may confirm it — the world's addon
// may be absent, the link may be down — and it must still appear, marked as unconfirmed,
// rather than being swallowed because delivery could not be proven.
describe('ChatManager optimistic send', () => {
    beforeEach(() => {
        mockInvoke({ chat_send: () => null, bedrock_send_chat: () => null });
    });

    test('a typed line appears immediately, marked unconfirmed', async () => {
        const manager = new ChatManager('Alaydriem');

        await manager.send('hello');

        const lines = get(manager.lines);
        expect(lines).toHaveLength(1);
        expect(lines[0].text).toBe('hello');
        expect(lines[0].delivery).toBe('pending');
    });

    // With no world known to the server there is nothing to address, and the old code
    // returned before sending or rendering anything at all.
    test('a line typed with no target still appears', async () => {
        const manager = new ChatManager('Alaydriem');

        await manager.send('nobody is listening');

        expect(get(manager.lines)).toHaveLength(1);
        expect(get(manager.lines)[0].delivery).toBe('pending');
    });

    test('a refused send is marked failed and stays visible', async () => {
        mockInvoke({
            chat_send: () => {
                throw new Error('Could not reach the server');
            },
        });
        const manager = new ChatManager('Alaydriem');

        await manager.send('into the void');

        const lines = get(manager.lines);
        expect(lines).toHaveLength(1);
        expect(lines[0].delivery).toBe('failed');
    });

    // The echo is the confirmation. Appending it would show the sender their own line twice.
    test('an echo of a pending line promotes it rather than duplicating it', async () => {
        const manager = new ChatManager('Alaydriem');
        await manager.send('hello');

        manager.acceptLine({ author: 'Alaydriem', text: 'hello', system: false });

        const lines = get(manager.lines);
        expect(lines).toHaveLength(1);
        expect(lines[0].delivery).toBe('confirmed');
    });

    // Somebody else saying the same words is not a confirmation of your send.
    test('another player saying the same words does not confirm your line', async () => {
        const manager = new ChatManager('Alaydriem');
        await manager.send('hello');

        manager.acceptLine({ author: 'SomebodyElse', text: 'hello', system: false });

        const lines = get(manager.lines);
        expect(lines).toHaveLength(2);
        expect(lines[0].delivery).toBe('pending');
    });

    // The reason has to reach the sender, and it has to settle the line they can see rather
    // than only raising a notice beside it.
    test('a server rejection marks the matching line failed and names the reason', async () => {
        const manager = new ChatManager('Alaydriem');
        await manager.send('hello');

        manager.handleRejection({
            reason: 'no chat channel is registered for this world',
            text: 'hello',
        });

        const lines = get(manager.lines);
        expect(lines).toHaveLength(1);
        expect(lines[0].delivery).toBe('failed');
        const rejection = get(manager.rejection);
        expect(rejection?.kind).toBe('failed');
        expect((rejection as { reason: string }).reason).toContain('no chat channel');
    });

    // Two sends in flight with different text: refusing one must not settle the other.
    test('a rejection settles only the line whose text it names', async () => {
        const manager = new ChatManager('Alaydriem');
        await manager.send('first');
        await manager.send('second');

        manager.handleRejection({ reason: 'no world was named', text: 'second' });

        const lines = get(manager.lines);
        expect(lines[0].delivery).toBe('pending');
        expect(lines[1].delivery).toBe('failed');
    });

    // Nothing answered at all. Left pending forever the sender would assume it landed.
    test('a line nothing answers is marked failed once the window closes', async () => {
        vi.useFakeTimers();
        try {
            const manager = new ChatManager('Alaydriem');
            await manager.send('hello');
            expect(get(manager.lines)[0].delivery).toBe('pending');

            vi.advanceTimersByTime(ChatManager.ANSWER_WINDOW_MS + 1);

            expect(get(manager.lines)[0].delivery).toBe('failed');
        } finally {
            vi.useRealTimers();
        }
    });
});

describe('ChatManager.resolveTarget', () => {
    test('in game targets the world the player is standing in', () => {
        const worlds = [world('w1', 'Survival'), world('w2', 'Creative')];

        const target = ChatManager.resolveTarget(worlds, 'w2');

        expect(target.kind).toBe('in-game');
        expect(target.kind === 'in-game' && target.world.world_name).toBe('Creative');
    });

    test('in game never offers a choice, even with several worlds available', () => {
        const worlds = [world('w1', 'Survival'), world('w2', 'Creative')];

        expect(ChatManager.resolveTarget(worlds, 'w1').kind).not.toBe('choose');
    });

    test('out of game with one known world targets it automatically', () => {
        const worlds = [world('w1', 'Survival')];

        const target = ChatManager.resolveTarget(worlds, null);

        expect(target.kind).toBe('only');
        expect(target.kind === 'only' && target.world.world_uuid).toBe('w1');
    });

    // A registered chat channel is a momentary reading and it flaps. The world the server
    // declared is still addressable, and the send path reports a real failure if the addon
    // turns out to be away — predicting one here is what made a typed line vanish.
    test('out of game offers a world whose chat channel is not registered', () => {
        const worlds = [world('w1', 'Survival', false)];

        const target = ChatManager.resolveTarget(worlds, null);

        expect(target.kind).toBe('only');
        expect(target.kind === 'only' && target.world.world_uuid).toBe('w1');
    });

    test('out of game with several usable worlds offers a choice', () => {
        const worlds = [world('w1', 'Survival'), world('w2', 'Creative')];

        const target = ChatManager.resolveTarget(worlds, null);

        expect(target.kind).toBe('choose');
        expect(target.kind === 'choose' && target.options).toHaveLength(2);
    });

    // The player is standing in w1, so w1 is where their message belongs whether or not its
    // chat channel answered a moment ago. What must never happen is falling through to w2,
    // and the standing-world lookup is what prevents that.
    test('a world the player is standing in is targeted even with no chat channel', () => {
        const worlds = [world('w1', 'Survival', false), world('w2', 'Creative')];

        const target = ChatManager.resolveTarget(worlds, 'w1');

        expect(target.kind).toBe('in-game');
        expect(target.kind === 'in-game' && target.world.world_uuid).toBe('w1');
    });

    test('no world to name at all is unavailable', () => {
        expect(ChatManager.resolveTarget([], null).kind).toBe('unavailable');
    });

    // First join into a world with no history row yet. Falling through to "some other world
    // is usable" would name a world the player is not in — the server would reject the send,
    // but only after the composer spent the whole time claiming it would land there.
    test('standing in a world that is not in the list is unavailable, not some other world', () => {
        const worlds = [world('w1', 'Survival'), world('w2', 'Creative')];

        const target = ChatManager.resolveTarget(worlds, 'brand-new-world');

        expect(target.kind).toBe('unavailable');
    });
});

/**
 * A world's reported name identifies nobody — a uuid from BDS, Paper's default `world`. What
 * the reader recognises is the name they picked in BVC Connect, so the pair is learned while
 * they are standing in it and kept for every later listing.
 */
describe('ChatManager world associations', () => {
    test('remembering a world exposes it for the label resolver', async () => {
        const manager = new ChatManager('Alaydriem');

        await manager.rememberWorld('w1', 'Truly Bedrock SMP');

        expect(get(manager.associations)['w1']).toBe('Truly Bedrock SMP');
    });

    test('a later name for the same world replaces the earlier one', async () => {
        const manager = new ChatManager('Alaydriem');

        await manager.rememberWorld('w1', 'Old Name');
        await manager.rememberWorld('w1', 'Renamed');

        expect(get(manager.associations)['w1']).toBe('Renamed');
    });

    test('worlds are remembered independently', async () => {
        const manager = new ChatManager('Alaydriem');

        await manager.rememberWorld('w1', 'One');
        await manager.rememberWorld('w2', 'Two');

        expect(get(manager.associations)).toEqual({ w1: 'One', w2: 'Two' });
    });
});

describe('ChatManager under a server that disabled chat', () => {
    beforeEach(() => {
        mockInvoke({
            chat_enabled: () => false,
            chat_worlds: () => [world('world-a', 'Overworld')],
            chat_transport: () => 'server',
            chat_send: () => null,
            bedrock_send_chat: () => null,
        });
    });

    test('the target reports disabled and the store says so', async () => {
        const manager = new ChatManager('Alaydriem');

        await manager.startLocal();

        expect(get(manager.target).kind).toBe('disabled');
        expect(get(manager.enabled)).toBe(false);
        await manager.stop();
    });

    // The policy must survive the poll that would otherwise resolve a live world a moment
    // later, or the dock states the reason and then contradicts itself.
    test('a live world does not overwrite the policy', async () => {
        vi.useFakeTimers();
        try {
            const manager = new ChatManager('Alaydriem');
            await manager.startLocal();

            await vi.advanceTimersByTimeAsync(ChatManager.AVAILABILITY_POLL_MS * 3);

            expect(get(manager.target).kind).toBe('disabled');
            await manager.stop();
        } finally {
            vi.useRealTimers();
        }
    });

    test('canSend is false', async () => {
        const manager = new ChatManager('Alaydriem');
        await manager.startLocal();

        expect(get(manager.canSend)).toBe(false);
        await manager.stop();
    });

    // The optimistic line exists so a sender can read back what nothing could carry. There is
    // no later moment when this one lands, so rendering it would be the misleading answer.
    test('a typed line is not rendered', async () => {
        const manager = new ChatManager('Alaydriem');
        await manager.startLocal();

        await manager.send('hello');

        expect(get(manager.lines)).toHaveLength(0);
        await manager.stop();
    });
});
