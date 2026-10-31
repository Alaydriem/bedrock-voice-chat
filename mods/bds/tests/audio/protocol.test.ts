import { test } from 'node:test';
import assert from 'node:assert/strict';
import { JukeboxBusProtocol } from '../../src/audio/sender/protocol';
import { Dimension } from '../../src/dto/dimension';

// The client proxy splits the play name on its first ':' after the prefix, so the rest of
// the name, colons included, is the dimension id.
test('a play name carries a custom dimension id whole', () => {
  assert.equal(
    JukeboxBusProtocol.playName('abc', Dimension.of('mypack:aether')),
    'bvc:play:abc:mypack:aether',
  );
});

test('an eject name carries the dimension id unchanged', () => {
  assert.equal(
    JukeboxBusProtocol.ejectName(Dimension.of('minecraft:nether')),
    'bvc:eject:minecraft:nether',
  );
});
