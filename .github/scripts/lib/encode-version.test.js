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

test('channel decodes the prerelease tag', () => {
  assert.deepStrictEqual(VersionEncoder.channel('1.0.0-alpha.3'), { name: 'alpha', number: 1 });
  assert.deepStrictEqual(VersionEncoder.channel('1.2.0-internal.1'), { name: 'internal', number: 2 });
  assert.deepStrictEqual(VersionEncoder.channel('1.0.0-beta.8'), { name: 'beta', number: 5 });
  assert.deepStrictEqual(VersionEncoder.channel('1.0.0-rc.2'), { name: 'rc', number: 8 });
});

test('channel treats a missing prerelease as stable', () => {
  assert.deepStrictEqual(VersionEncoder.channel('1.1.0'), { name: 'stable', number: 9 });
});

test('channel treats an unrecognized prerelease as stable', () => {
  assert.deepStrictEqual(VersionEncoder.channel('1.0.0-foo.1'), { name: 'stable', number: 9 });
});

test('channel handles a prerelease tag without a number', () => {
  assert.deepStrictEqual(VersionEncoder.channel('1.0.0-internal'), { name: 'internal', number: 2 });
});

test('flagsmithEnvironment maps alpha and internal to dev', () => {
  assert.strictEqual(VersionEncoder.flagsmithEnvironment('1.0.0-alpha.3'), 'dev');
  assert.strictEqual(VersionEncoder.flagsmithEnvironment('1.0.0-internal.1'), 'dev');
});

test('flagsmithEnvironment maps beta and rc to staging', () => {
  assert.strictEqual(VersionEncoder.flagsmithEnvironment('1.0.0-beta.8'), 'staging');
  assert.strictEqual(VersionEncoder.flagsmithEnvironment('1.0.0-rc.2'), 'staging');
});

test('flagsmithEnvironment maps stable (and unrecognized) to prod', () => {
  assert.strictEqual(VersionEncoder.flagsmithEnvironment('1.1.0'), 'prod');
  assert.strictEqual(VersionEncoder.flagsmithEnvironment('1.0.0-foo.1'), 'prod');
});
