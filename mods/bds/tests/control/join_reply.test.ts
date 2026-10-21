import { test } from 'node:test';
import assert from 'node:assert/strict';
import { JoinReply } from '../../src/control/join_reply';

test('a protected join handed to the app points the player at the app', () => {
  assert.equal(
    JoinReply.forResult({ ok: true, outcome: 'forwarded' }, 'Night Raid', true),
    '§e[BVC] Your code for Night Raid is in the BVC app.',
  );
});

test('a protected join with no app open tells the player to open it', () => {
  assert.equal(
    JoinReply.forResult({ ok: false, outcome: 'no_session' }, 'Night Raid', true),
    '§c[BVC] Open the BVC app to join Night Raid.',
  );
});

// No-net mode has no reply: the proxy is the player's own app, which asks the members. The
// player must never read "Joined" for a group they are only asking to join.
test('a no-net protected join never says Joined', () => {
  assert.equal(
    JoinReply.forResult({ ok: true }, 'Night Raid', true),
    '§e[BVC] Your code for Night Raid is in the BVC app.',
  );
});

test('an open join says Joined', () => {
  assert.equal(JoinReply.forResult({ ok: true }, 'Build', false), '§a[BVC] Joined §fBuild');
});

test('a failed open join says so', () => {
  assert.equal(
    JoinReply.forResult({ ok: false }, 'Build', false),
    '§c[BVC] Group join failed; try again',
  );
});
