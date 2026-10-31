import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Dimension } from '../../src/dto/dimension';

// The server canonicalises and hashes; the mod must send the id exactly as Bedrock reports it,
// or two paths for one dimension would hash differently.
test('a custom id is sent unchanged', () => {
  assert.equal(Dimension.of('mypack:aether').toApiString(), 'mypack:aether');
});

// A BVC server older than the pack accepts only the bare vanilla names, and rejects the
// whole position batch on one it does not know. Vanilla ids are never hashed, so dropping
// the prefix costs nothing.
test('a vanilla id is sent in the bare form every server accepts', () => {
  assert.equal(Dimension.of('minecraft:overworld').toApiString(), 'overworld');
  assert.equal(Dimension.of('minecraft:nether').toApiString(), 'nether');
  assert.equal(Dimension.of('minecraft:the_end').toApiString(), 'the_end');
});

test('a custom id in the minecraft namespace keeps its prefix', () => {
  assert.equal(Dimension.of('minecraft:sky_islands').toApiString(), 'minecraft:sky_islands');
});

test('only non-vanilla ids are custom', () => {
  assert.equal(Dimension.of('mypack:aether').isCustom(), true);
  for (const id of ['minecraft:overworld', 'minecraft:nether', 'minecraft:the_end']) {
    assert.equal(Dimension.of(id).isCustom(), false, id);
  }
  assert.equal(Dimension.DEATH.isCustom(), false);
});

test('two dimensions are the same only when their ids match', () => {
  assert.equal(Dimension.of('mypack:aether').equals(Dimension.of('mypack:aether')), true);
  assert.equal(Dimension.of('mypack:aether').equals(Dimension.of('other:aether')), false);
});
