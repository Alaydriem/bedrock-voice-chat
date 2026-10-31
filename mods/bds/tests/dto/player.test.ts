import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Coordinates } from '../../src/dto/coordinates';
import { Dimension } from '../../src/dto/dimension';
import { Orientation } from '../../src/dto/orientation';
import { Player } from '../../src/dto/player';

test('a player in a custom dimension posts its raw id', () => {
  const player = new Player(
    'Alex',
    Dimension.of('mypack:aether'),
    new Coordinates(1, 64, -3),
    false,
    new Orientation(0, 0),
  );

  assert.equal(player.toJSON().dimension, 'mypack:aether');
});

// A disconnected player is parked in "death" so no proximity check can route their audio.
test('a disconnected player is still parked in death', () => {
  assert.equal(Player.fromDisconnectedPlayer('Alex').toJSON().dimension, 'death');
});
