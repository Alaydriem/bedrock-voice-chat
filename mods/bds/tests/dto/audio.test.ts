import { test } from 'node:test';
import assert from 'node:assert/strict';
import { AudioPlayRequest } from '../../src/dto/audio';
import { Coordinates } from '../../src/dto/coordinates';
import { Dimension } from '../../src/dto/dimension';

test('a jukebox in a custom dimension requests playback with its raw id', () => {
  const request = new AudioPlayRequest(
    'disc-1',
    new Coordinates(1, 64, -3),
    Dimension.of('mypack:aether'),
    'world-1',
  );

  const json = request.toJSON() as { game: { dimension: string } };
  assert.equal(json.game.dimension, 'mypack:aether');
});
