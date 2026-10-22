import { test } from 'node:test';
import assert from 'node:assert/strict';
import { GroupRowParser } from '../../src/state/group_row_parser';

test('a net row marked protected maps to a protected row', () => {
  const rows = GroupRowParser.parse(
    JSON.stringify([
      { id: 'NR4K', name: 'Night Raid', protected: true, players: [], creator: 'x' },
      { id: 'OPEN', name: 'Open', players: [], creator: 'x' },
    ]),
  );

  assert.deepEqual(rows, [
    { id: 'NR4K', name: 'Night Raid', protected: true },
    { id: 'OPEN', name: 'Open', protected: false },
  ]);
});

test('a body that is not a list reads as unavailable', () => {
  assert.equal(GroupRowParser.parse('{"error":true}'), null);
  assert.equal(GroupRowParser.parse('not json'), null);
});
