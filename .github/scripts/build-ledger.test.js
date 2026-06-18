const test = require('node:test');
const assert = require('node:assert');
const { BuildLedger } = require('./build-ledger');

test('record builds a normalized entry with an integer build_number', () => {
  const r = BuildLedger.record(1001084, {
    sha: 'abc123', branch: 'beta', ref: 'refs/heads/beta',
    version: '1.0.0-beta.13', track: 'alpha',
    run: 'https://example/run/1', ts: '2026-06-17T00:00:00Z',
  });
  assert.strictEqual(r.build_number, 1001084);
  assert.strictEqual(typeof r.build_number, 'number');
  assert.strictEqual(r.track, 'alpha');
});

test('toJson sorts by build_number descending and parses HGETALL values', () => {
  const hgetall = {
    '1001084': JSON.stringify({ build_number: 1001084, track: 'alpha' }),
    '1001090': JSON.stringify({ build_number: 1001090, track: 'beta' }),
  };
  const json = BuildLedger.toJson(hgetall);
  const parsed = JSON.parse(json);
  assert.strictEqual(parsed.builds[0].build_number, 1001090);
  assert.strictEqual(parsed.builds[1].build_number, 1001084);
});
