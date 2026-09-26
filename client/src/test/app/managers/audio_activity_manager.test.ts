import { describe, expect, test } from 'vitest';
import { get } from 'svelte/store';
import { AudioActivityManager } from '../../../js/app/managers/AudioActivityManager';

const store = {} as never;

const PEER = 'minecraft:VoxelWren';

describe('AudioActivityManager', () => {
    test('drops a speaker whose last activity is older than the highlight window', () => {
        const manager = new AudioActivityManager(store);
        manager.applyLevels({ [PEER]: 0.5 }, 1_000);

        manager.prune(3_000);

        expect(get(manager.audioActivity).activeSpeakers[PEER]).toBeUndefined();
    });

    test('keeps a speaker who is still inside the highlight window', () => {
        const manager = new AudioActivityManager(store);
        manager.applyLevels({ [PEER]: 0.5 }, 1_000);

        manager.prune(1_500);

        expect(get(manager.audioActivity).activeSpeakers[PEER]).toBeDefined();
    });

    test('a fade that lands after a prune does not resurrect the speaker', () => {
        const manager = new AudioActivityManager(store);
        manager.applyLevels({ [PEER]: 0.5 }, 1_000);
        manager.prune(3_000);

        manager.fade(PEER);

        expect(get(manager.audioActivity).activeSpeakers[PEER]).toBeUndefined();
    });
});
