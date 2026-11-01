import { test } from 'node:test';
import assert from 'node:assert/strict';
import { GameMode } from '@minecraft/server';
import type { Player as MinecraftPlayer } from '@minecraft/server';
import { Player } from '../../src/dto/player';

function inDimension(dimensionId: string): MinecraftPlayer {
  return {
    name: 'Alex',
    dimension: { id: dimensionId },
    location: { x: 1, y: 64, z: -3 },
    isSneaking: false,
    getRotation: () => ({ x: 0, y: 0 }),
    getGameMode: () => GameMode.Survival,
  } as unknown as MinecraftPlayer;
}

// The server hashes a custom id into a dimension of its own, so the id must arrive whole.
test('a player in a custom dimension posts its id whole', () => {
  const json = Player.fromMinecraftPlayer(inDimension('mypack:aether')).toJSON();

  assert.equal(json.dimension, 'mypack:aether');
});

// The bare form is the one every server version accepts.
test('a player in a vanilla dimension posts the bare name', () => {
  const json = Player.fromMinecraftPlayer(inDimension('minecraft:the_end')).toJSON();

  assert.equal(json.dimension, 'the_end');
});

// The server strips one leading `minecraft:` before it hashes, so this id names the same
// dimension on the server whether or not the prefix is sent.
test('a custom id in the minecraft namespace posts without the prefix', () => {
  const json = Player.fromMinecraftPlayer(inDimension('minecraft:sky_islands')).toJSON();

  assert.equal(json.dimension, 'sky_islands');
});

// Death overrides the dimension the player died in, custom or not.
test('a dead player in a custom dimension is parked in death', () => {
  const json = Player.fromMinecraftPlayerDead(inDimension('mypack:aether')).toJSON();

  assert.equal(json.dimension, 'death');
});

// A disconnected player is parked in "death" so no proximity check can route their audio.
test('a disconnected player is parked in death', () => {
  assert.equal(Player.fromDisconnectedPlayer('Alex').toJSON().dimension, 'death');
});
