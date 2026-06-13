const test = require('node:test');
const assert = require('node:assert');
const { VersionEncoder } = require('./encode-version');

test('internal maps to channel 2', () => {
  assert.strictEqual(VersionEncoder.encode('1.0.0-internal.4').display, '1.0.204');
});

test('internal scales with patch/major/minor', () => {
  assert.strictEqual(VersionEncoder.encode('2.1.3-internal.4').display, '2.1.3204');
});

test('existing channels are unchanged', () => {
  assert.strictEqual(VersionEncoder.encode('1.0.0-alpha.3').display, '1.0.103');
  assert.strictEqual(VersionEncoder.encode('1.0.0-beta.7').display, '1.0.507');
  assert.strictEqual(VersionEncoder.encode('1.0.0-rc.2').display, '1.0.802');
  assert.strictEqual(VersionEncoder.encode('1.0.0').display, '1.0.900');
});

test('internal without an explicit number defaults to 1', () => {
  assert.strictEqual(VersionEncoder.encode('1.0.0-internal').display, '1.0.201');
});

test('versionCode flattens to a single integer', () => {
  assert.strictEqual(VersionEncoder.versionCode('1.0.0-beta.8'), 1000508);
  assert.strictEqual(VersionEncoder.versionCode('1.0.0-internal.4'), 1000204);
});
