// One command that runs a full jukebox round trip against a real server, client and BDS, and
// asserts audio was actually decoded rather than that the events merely fired.
//
// Two modes, because the two senders reach the server by completely different routes:
//
//   --net     (default) Deploys the full pack. `NetAudioSender` POSTs /api/audio/event itself,
//             and it ignores the acting player entirely, so no Bedrock client has to be joined.
//             This script posts the listener's position instead — the addon's own position loop
//             is gated behind `bvc_minimum_players` and never runs with an empty world, and a
//             listener the server has no position for is dropped by proximity routing with no
//             log line. Fully automated.
//
//   --no-net  Deploys the no-net pack. `NoNetAudioSender` emits a CLIENTBOUND PlaySound, which
//             only exists if there is a client to address, so a real Bedrock client has to join
//             the world. The script pauses and asks.
//
// The disc goes in through `scriptevent bvc:insert`, which dispatches to whichever sender the
// deployed pack selected. Playing through the addon is the point: posting /api/audio/event
// directly would prove the server works and the addon nothing.
//
// The clip is read from the dev server's SQLite file, because GET /api/audio/file authenticates
// with a client certificate that Node cannot present.
//
//   yarn jukebox-test --net --gamertag Alaydriem
//   yarn jukebox-test --net --gamertag Alaydriem --at 12,70,-30
//   yarn jukebox-test --net --gamertag Alaydriem --no-build
//   yarn jukebox-test --no-net
//
// Requires Node 22.5+ for `node:sqlite`. Exits 0 on PASS, 1 on FAIL.

const { spawn } = require('child_process');
const path = require('path');
const readline = require('readline');
const { existsSync, readFileSync } = require('fs');

const { loadEnv, requireEnv, buildAndDeployBds } = require('./lib/bds');

const DEFAULT_JUKEBOX_AT = { x: 0, y: 64, z: 0 };
const CLIENT_WS = 'ws://127.0.0.1:9595';
const FRAME_WAIT_MS = 20_000;
const SETTLE_MS = 3_000;
const POSITION_INTERVAL_MS = 1_000;
const BDS_READY_MS = 120_000;

const args = process.argv.slice(2);
const flags = new Set(
  args.filter((a) => a.startsWith('--')).map((a) => a.replace(/^--/, ''))
);

function argValue(name) {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : undefined;
}

function step(message) {
  console.log(`\n[jukebox-test] ${message}`);
}

function fail(message) {
  console.error(`\n[jukebox-test] FAIL: ${message}`);
  process.exit(1);
}

function ask(question) {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  return new Promise((resolve) =>
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer);
    })
  );
}

// Where the jukebox goes. The listener stands one block east of it, which is well inside any
// broadcast range and keeps the two in the same chunk.
function jukeboxLocation() {
  const raw = argValue('at');
  if (!raw) {
    return DEFAULT_JUKEBOX_AT;
  }

  const parts = raw
    .split(/[\s,]+/)
    .filter((p) => p.length > 0)
    .map((p) => Number.parseInt(p, 10));

  if (parts.length !== 3 || parts.some((n) => Number.isNaN(n))) {
    fail(`--at must be three integers, e.g. --at 12,70,-30 (got "${raw}")`);
  }

  return { x: parts[0], y: parts[1], z: parts[2] };
}

// The credentials the addon itself reads, taken from the same file so the script and the addon
// cannot end up pointed at different servers.
function serverConfig(bdsRoot) {
  const file = path.join(bdsRoot, 'config', 'default', 'variables.json');
  if (!existsSync(file)) {
    fail(
      `no ${file}.\n` +
        '       Net mode needs the addon configured with bvc_server and bvc_access_token.'
    );
  }

  let parsed;
  try {
    parsed = JSON.parse(readFileSync(file, 'utf8'));
  } catch (e) {
    fail(`could not parse ${file}: ${e.message}`);
  }

  if (!parsed.bvc_server || !parsed.bvc_access_token) {
    fail(`${file} is missing bvc_server or bvc_access_token`);
  }

  return { server: parsed.bvc_server, token: parsed.bvc_access_token };
}

// The clip to play, newest first — the one an operator just uploaded is the one they mean.
// `deleted` is a soft-delete flag, so a row with it set may have no `.opus` left on disk.
function resolveAudioId(env) {
  const explicit = argValue('audio-id');
  if (explicit) {
    return explicit;
  }

  const { DatabaseSync } = require('node:sqlite');
  const dbPath = env.BVC_DATABASE
    ? path.resolve(env.BVC_DATABASE)
    : path.join(__dirname, '..', 'server', 'server', 'bvc.sqlite3');

  if (!existsSync(dbPath)) {
    fail(
      `no database at ${dbPath}.\n` +
        '       Set BVC_DATABASE in mods/.env, or pass --audio-id <id> to skip the lookup.'
    );
  }

  const db = new DatabaseSync(dbPath, { readOnly: true });
  try {
    const row = db
      .prepare(
        'SELECT id, original_filename, duration_ms FROM audio_file ' +
          'WHERE deleted = 0 ORDER BY created_at DESC LIMIT 1'
      )
      .get();

    if (!row) {
      fail(
        "no audio files in the library; upload one in the client's Settings > Library first"
      );
    }

    console.log(
      `[jukebox-test] using "${row.original_filename}" (${row.duration_ms}ms) id=${row.id}`
    );
    return row.id;
  } finally {
    db.close();
  }
}

function wsUrl(base, key) {
  return key ? `${base}?key=${encodeURIComponent(key)}` : base;
}

// Stands the listener next to the jukebox, and keeps standing them there.
//
// `world_uuid` is deliberately omitted. `can_communicate_with` only compares worlds when both
// sides carry one, so leaving it off matches whatever the addon stamped on the jukebox without
// having to discover the addon's generated id — and it avoids marking the addon HTTP-healthy as
// a side effect of a test posting positions.
function startPositionPoster(config, gamertag, at) {
  const body = JSON.stringify({
    game: 'minecraft',
    players: [
      {
        name: gamertag,
        dimension: 'overworld',
        coordinates: { x: at.x + 1, y: at.y, z: at.z },
        deafen: false,
        orientation: { x: 0, y: 0 },
        spectator: false,
      },
    ],
  });

  let failures = 0;
  const post = async () => {
    try {
      const res = await fetch(`${config.server}/api/position`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-MC-Access-Token': config.token,
          Accept: 'application/json',
        },
        body,
      });
      if (!res.ok && failures++ === 0) {
        console.warn(
          `[jukebox-test] /api/position returned ${res.status}; the listener may not be routable`
        );
      }
    } catch (e) {
      if (failures++ === 0) {
        console.warn(`[jukebox-test] /api/position unreachable: ${e.message}`);
      }
    }
  };

  void post();
  const timer = setInterval(post, POSITION_INTERVAL_MS);
  return () => clearInterval(timer);
}

// Points the client at the world over the socket rather than by clicking. No-net only: net mode
// needs no Bedrock proxy at all, only the client's existing QUIC link to the BVC server.
async function connectClient(env, targetName) {
  const WebSocket = require('ws');
  const ws = new WebSocket(CLIENT_WS);
  await new Promise((resolve, reject) => {
    ws.once('open', resolve);
    ws.once('error', () =>
      reject(
        new Error(
          `no client listening on ${CLIENT_WS}. Start the BVC client and enable its WebSocket in Settings.`
        )
      )
    );
  });

  const send = (payload) =>
    new Promise((resolve, reject) => {
      ws.once('message', (raw) => {
        const parsed = JSON.parse(raw.toString());
        if (parsed.success) {
          resolve(parsed.data);
        } else {
          reject(new Error(parsed.error ?? 'command failed'));
        }
      });
      ws.send(JSON.stringify({ ...payload, key: env.BVC_WS_KEY }));
    });

  const { targets } = await send({ action: 'targets' });
  const target = targets.find(
    (t) => t.kind === 'proxy' && (!targetName || t.name === targetName)
  );
  if (!target) {
    ws.close();
    fail(
      `no saved proxy target${targetName ? ` named "${targetName}"` : ''}; add one in the client first`
    );
  }

  const result = await send({ action: 'connect', id: target.id });
  ws.close();
  return result;
}

// Watches the client's own diagnostics for a jukebox that decoded frames.
//
// This is the assertion the whole script exists for: every event can be correct while nothing is
// audible, which is exactly the failure this was written after.
function watchForJukeboxAudio(env, timeoutMs) {
  const WebSocket = require('ws');
  const ws = new WebSocket(wsUrl(`${CLIENT_WS}/metrics`, env.BVC_WS_KEY));
  let sawSnapshot = false;

  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      ws.close();
      resolve({ jukebox: null, sawSnapshot });
    }, timeoutMs);

    ws.on('message', (raw) => {
      let push;
      try {
        push = JSON.parse(raw.toString());
      } catch {
        return;
      }

      // MetricsPush is { type: "metrics", data: LinkDiagnosticsSnapshot }, and the snapshot
      // carries `peers: PeerDiagnostics[]`.
      if (push.type !== 'metrics') {
        return;
      }
      sawSnapshot = true;

      const peers = push.data?.peers ?? [];
      const jukebox = peers.find(
        (p) => p.name.startsWith('jukebox-') && p.frames_decoded > 0
      );
      if (jukebox) {
        clearTimeout(timer);
        ws.close();
        resolve({ jukebox, sawSnapshot });
      }
    });

    ws.on('error', () => {
      clearTimeout(timer);
      resolve({ jukebox: null, sawSnapshot });
    });
  });
}

// Owns BDS so its console is writable. stdout is piped and echoed rather than inherited: the
// script has to know when the server is accepting commands, and a fixed sleep would either be
// too short on a cold world or waste time on a warm one.
function startBds(bdsRoot) {
  const exe = path.join(bdsRoot, 'bedrock_server.exe');
  if (!existsSync(exe)) {
    fail(`bedrock_server.exe not found in ${bdsRoot}`);
  }

  const proc = spawn(exe, [], {
    cwd: bdsRoot,
    stdio: ['pipe', 'pipe', 'inherit'],
  });

  let buffered = '';
  const waiters = [];
  proc.stdout.on('data', (chunk) => {
    const text = chunk.toString();
    process.stdout.write(text);
    buffered += text;
    for (const waiter of [...waiters] ) {
      if (waiter.pattern.test(buffered)) {
        waiters.splice(waiters.indexOf(waiter), 1);
        waiter.resolve(true);
      }
    }
  });

  const waitFor = (pattern, timeoutMs) =>
    new Promise((resolve) => {
      if (pattern.test(buffered)) {
        resolve(true);
        return;
      }
      const waiter = { pattern, resolve };
      waiters.push(waiter);
      setTimeout(() => {
        const i = waiters.indexOf(waiter);
        if (i >= 0) {
          waiters.splice(i, 1);
          resolve(false);
        }
      }, timeoutMs);
    });

  return { proc, waitFor };
}

function consoleCommand(bds, command) {
  console.log(`[jukebox-test] > ${command}`);
  bds.proc.stdin.write(`${command}\n`);
}

async function main() {
  const env = loadEnv();
  const bdsRoot = requireEnv(env, 'BDS_SERVER_PATH');

  const noNet = flags.has('no-net');
  const gamertag = argValue('gamertag');
  if (!noNet && !gamertag) {
    fail(
      '--gamertag <name> is required in net mode.\n' +
        "       It is the bare Xbox gamertag the client authenticated as; the script posts the\n" +
        '       listener position under it so the server can route jukebox audio to that client.'
    );
  }

  const at = jukeboxLocation();
  const audioId = resolveAudioId(env);
  const config = noNet ? null : serverConfig(bdsRoot);

  if (!flags.has('no-build')) {
    buildAndDeployBds(bdsRoot, noNet);
  }

  if (noNet) {
    step('connecting the BVC client to the proxy target');
    const connected = await connectClient(env, argValue('target'));
    console.log(`[jukebox-test] client connected to "${connected.name}"`);
  }

  step('starting BDS');
  const bds = startBds(bdsRoot);
  let stopPoster = null;
  const cleanup = () => {
    if (stopPoster) stopPoster();
    try {
      bds.proc.stdin.write('stop\n');
    } catch {
      // Already gone; nothing to stop.
    }
  };
  process.on('SIGINT', () => {
    cleanup();
    process.exit(130);
  });

  const ready = await bds.waitFor(/Server started/i, BDS_READY_MS);
  if (!ready) {
    cleanup();
    fail(`BDS did not report "Server started" within ${BDS_READY_MS / 1000}s`);
  }

  if (noNet) {
    await ask(
      '\n[jukebox-test] Join the world through the BVC proxy (127.0.0.1:19137), stand anywhere,\n' +
        '               then press Enter here. '
    );
  } else {
    step(`posting ${gamertag}'s position beside the jukebox`);
    stopPoster = startPositionPoster(config, gamertag, at);
  }

  step(`placing the jukebox at (${at.x},${at.y},${at.z})`);
  // A ticking area rather than a nearby player: in net mode nobody is in the world, and the
  // block component and the chunk it lives in have to tick for a disc to play at all.
  consoleCommand(bds, `tickingarea add circle ${at.x} ${at.y} ${at.z} 2 bvcjukeboxtest`);
  consoleCommand(bds, `setblock ${at.x} ${at.y} ${at.z} bvc:audio_player`);
  if (noNet) {
    consoleCommand(bds, `tp @a ${at.x + 1} ${at.y} ${at.z}`);
  }
  await new Promise((r) => setTimeout(r, SETTLE_MS));

  step('inserting the disc and watching for decoded audio');
  const watching = watchForJukeboxAudio(env, FRAME_WAIT_MS);
  consoleCommand(bds, `scriptevent bvc:insert ${audioId} ${at.x} ${at.y} ${at.z}`);
  const { jukebox, sawSnapshot } = await watching;

  if (!jukebox) {
    consoleCommand(bds, `scriptevent bvc:eject ${at.x} ${at.y} ${at.z}`);
    cleanup();

    if (!sawSnapshot) {
      fail(
        `no diagnostics arrived from the client on ${CLIENT_WS}/metrics.\n` +
          '       The client is not running, its WebSocket is off, or BVC_WS_KEY in mods/.env is wrong.'
      );
    }

    fail(
      `the client is reporting, but no jukebox peer decoded a frame within ${FRAME_WAIT_MS / 1000}s.\n` +
        "       The events may all be correct and the audio still silent — that is this test's\n" +
        '       whole point. Check the server for "Starting audio playback", then the client for\n' +
        '       "Creating jitter buffer ... jukebox-". Playback running with no jitter buffer means\n' +
        `       route_audio_frame dropped every frame${noNet ? '' : `; confirm ${gamertag} is the identity this client authenticated as`}.`
    );
  }

  console.log(
    `[jukebox-test] heard ${jukebox.name}: frames_decoded=${jukebox.frames_decoded} ` +
      `quality=${jukebox.quality_score} concealment=${jukebox.concealment_pct}%`
  );

  step('ejecting');
  consoleCommand(bds, `scriptevent bvc:eject ${at.x} ${at.y} ${at.z}`);
  await new Promise((r) => setTimeout(r, SETTLE_MS));

  step(`PASS — inserted, heard, ejected (${noNet ? 'no-net' : 'net'} mode)`);
  cleanup();
  process.exit(0);
}

main().catch((e) => fail(e.message ?? String(e)));
