import { describe, expect, test } from 'vitest';
import { writable } from 'svelte/store';
import { PlayerManager } from '../../../js/app/managers/PlayerManager';
import { VoiceRosterPublisher } from '../../../js/app/managers/VoiceRosterPublisher';
import type { NearbyPlayer } from '../../../js/app/dashboard/NearbyPlayer';
import type { PresenceKind } from '../../../js/bindings/PresenceKind';

function manager(): PlayerManager {
    return new PlayerManager('Alaydriem', 'minecraft');
}

function near(gamertag: string, presence: PresenceKind, inEarshot = true): NearbyPlayer {
    return {
        name: `minecraft:${gamertag}`,
        gamertag,
        game: 'minecraft',
        hue: '#8239d8',
        presence,
        distance: inEarshot ? 10 : 200,
        bearing: 0,
        elevation: 0,
        inEarshot,
    };
}

describe('VoiceRosterPublisher.project', () => {
    /**
     * The position feed is the authority for who is near. PlayerManager's proximity source is
     * fed by audio-driven presence events, which a client that joined after a player never
     * receives — so a roster built from it is empty while four people are audible.
     */
    test('takes proximity members from the position feed', () => {
        const nearby = writable<readonly NearbyPlayer[]>([
            near('DiamondDelver', 'voice'),
            near('PistonPip', 'voice'),
        ]);

        const roster = new VoiceRosterPublisher(manager(), nearby).project();

        expect(roster.members.map(m => m.name)).toEqual([
            'minecraft:DiamondDelver',
            'minecraft:PistonPip',
        ]);
    });

    /** A player in the world with no voice connection hears nothing and is not on the roster. */
    test('leaves out a player who is present in the game but not on voice', () => {
        const nearby = writable<readonly NearbyPlayer[]>([
            near('DiamondDelver', 'voice'),
            near('NoVoiceNed', 'game'),
        ]);

        const roster = new VoiceRosterPublisher(manager(), nearby).project();

        expect(roster.members.map(m => m.name)).toEqual(['minecraft:DiamondDelver']);
    });

    /** Group audio carries at any distance, so the feed's range test would wrongly drop them. */
    test('keeps a group member who is out of earshot', async () => {
        const players = manager();
        await players.addPlayerSource('FarAwayFran', 'Group');
        const nearby = writable<readonly NearbyPlayer[]>([]);

        const roster = new VoiceRosterPublisher(players, nearby).project();

        expect(roster.members.map(m => m.name)).toEqual(['minecraft:FarAwayFran']);
        expect(roster.members[0].sources).toEqual(['Group']);
    });

    test('carries both sources for a group member who is also in earshot', async () => {
        const players = manager();
        await players.addPlayerSource('VoxelWren', 'Group');
        const nearby = writable<readonly NearbyPlayer[]>([near('VoxelWren', 'voice')]);

        const roster = new VoiceRosterPublisher(players, nearby).project();

        expect(roster.members).toHaveLength(1);
        expect([...roster.members[0].sources].sort()).toEqual(['Group', 'Proximity']);
    });

    test('reports the signed-in player as own and never as a member', () => {
        const nearby = writable<readonly NearbyPlayer[]>([
            near('Alaydriem', 'voice'),
            near('VoxelWren', 'voice'),
        ]);

        const roster = new VoiceRosterPublisher(manager(), nearby).project();

        expect(roster.own).toBe('minecraft:Alaydriem');
        expect(roster.members.map(m => m.name)).toEqual(['minecraft:VoxelWren']);
    });

    test('sorts members by name so tiles do not move while people talk', () => {
        const nearby = writable<readonly NearbyPlayer[]>([
            near('VoxelWren', 'voice'),
            near('CreeperCoda', 'voice'),
        ]);

        const roster = new VoiceRosterPublisher(manager(), nearby).project();

        expect(roster.members.map(m => m.name)).toEqual([
            'minecraft:CreeperCoda',
            'minecraft:VoxelWren',
        ]);
    });
});
