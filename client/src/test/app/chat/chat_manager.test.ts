import { describe, expect, test } from 'vitest';
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

    test('out of game with one usable world targets it automatically', () => {
        const worlds = [world('w1', 'Survival'), world('w2', 'Creative', false)];

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

    // Standing in a world whose chat channel is down must not fall through to some other
    // world. Sending there would put the message in front of people the player is not with.
    test('a world the player is standing in with no chat channel is unavailable', () => {
        const worlds = [world('w1', 'Survival', false), world('w2', 'Creative')];

        const target = ChatManager.resolveTarget(worlds, 'w1');

        expect(target.kind).toBe('unavailable');
    });

    test('no usable world at all is unavailable', () => {
        expect(ChatManager.resolveTarget([], null).kind).toBe('unavailable');
    });
});
