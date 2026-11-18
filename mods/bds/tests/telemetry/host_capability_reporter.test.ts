import { test } from 'node:test';
import assert from 'node:assert/strict';
import { HostCapabilityReporter } from '../../src/telemetry/host_capability_reporter';

test('sends_once_with_bds_loader_and_external_mode', async () => {
  const calls: Array<[string, string]> = [];
  const reporter = new HostCapabilityReporter(
    async (path, body) => {
      calls.push([path, body]);
    },
    () => true,
    '1.2.3',
  );

  await reporter.report();
  await reporter.report();

  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], '/api/telemetry/host-capability');
  assert.deepEqual(JSON.parse(calls[0][1]), {
    mod_version: '1.2.3',
    loader: 'bds',
    mode: 'external',
  });
});

test('sends_nothing_when_telemetry_is_off', async () => {
  let calls = 0;
  const reporter = new HostCapabilityReporter(
    async () => {
      calls++;
    },
    () => false,
    '1.2.3',
  );

  await reporter.report();

  assert.equal(calls, 0);
});

test('a_failed_post_does_not_throw', async () => {
  const reporter = new HostCapabilityReporter(
    async () => {
      throw new Error('unreachable');
    },
    () => true,
    '1.2.3',
  );

  await assert.doesNotReject(reporter.report());
});
