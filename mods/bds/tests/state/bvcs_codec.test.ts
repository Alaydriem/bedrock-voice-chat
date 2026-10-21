import { test } from 'node:test';
import assert from 'node:assert/strict';
import { BvcsCodec } from '../../src/state/bvcs_codec';

test('a protected group row decodes as protected', () => {
  assert.deepEqual(
    BvcsCodec.decode('!bvcs:4:g:i=NR4K;n=Night%20Raid;p=1'),
    { kind: 'g', id: 'NR4K', name: 'Night Raid', protected: true },
  );
});

test('a row without p is an open group', () => {
  assert.deepEqual(BvcsCodec.decode('!bvcs:4:g:i=NR4K;n=Night Raid'), {
    kind: 'g',
    id: 'NR4K',
    name: 'Night Raid',
    protected: false,
  });
});
