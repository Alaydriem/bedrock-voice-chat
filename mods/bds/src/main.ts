import { world, system } from '@minecraft/server';
import {
  HttpHeader,
  HttpRequest,
  HttpRequestMethod,
  http,
} from '@minecraft/server-net';
import { variables } from '@minecraft/server-admin';
import { Payload } from './dto';

const bvc_server: string = variables.get('bvc_server');
const access_token: string = variables.get('bvc_access_token');
const debug: boolean = variables.get("bvc_debug");

const POLL_INTERVAL = 5;
const MIN_PLAYERS = 2;
const REQUEST_TIMEOUT = 1;

const FAILURE_THRESHOLD = 3;
const INITIAL_BACKOFF_MS = 10_000;
const MAX_BACKOFF_MS = 30_000;
let consecutiveFailures = 0;
let circuitOpenUntil = 0;

// Track dead players by their ID
const deadPlayers = new Set<string>();

// Persistent world UUID for multi-world isolation
let cachedWorldUuid: string | undefined;

function getWorldUuid(): string {
  if (cachedWorldUuid) {
    return cachedWorldUuid;
  }

  const existing = world.getDynamicProperty('bvc:world_uuid');
  if (typeof existing === 'string') {
    cachedWorldUuid = existing;
    console.info("[BVC] Loaded world UUID: " + existing);
    return existing;
  }

  // Generate a UUID v4
  const uuid = [
    randomHex(8),
    randomHex(4),
    '4' + randomHex(3),
    ((Math.floor(Math.random() * 4) + 8).toString(16)) + randomHex(3),
    randomHex(12),
  ].join('-');

  world.setDynamicProperty('bvc:world_uuid', uuid);
  cachedWorldUuid = uuid;
  console.info("[BVC] Generated world UUID: " + uuid);
  return uuid;
}

function randomHex(length: number): string {
  let result = '';
  for (let i = 0; i < length; i++) {
    result += Math.floor(Math.random() * 16).toString(16);
  }
  return result;
}

console.info("[BVC] Connecting to: " + bvc_server);

// Subscribe to player death events
world.afterEvents.entityDie.subscribe(
  (event) => {
    const deadEntity = event.deadEntity;
    if (deadEntity.typeId === 'minecraft:player') {
      deadPlayers.add(deadEntity.id);
    }
  },
  { entityTypes: ['minecraft:player'] }
);

// Subscribe to player spawn events (includes respawns)
world.afterEvents.playerSpawn.subscribe((event) => {
  deadPlayers.delete(event.player.id);
});

system.runInterval(async () => {
  const players = world.getAllPlayers();

  if (!debug) {
    if (players.length < MIN_PLAYERS) {
      return;
    }
  }

  // Circuit breaker: skip requests while circuit is open
  const now = Date.now();
  if (consecutiveFailures >= FAILURE_THRESHOLD && now < circuitOpenUntil) {
    return;
  }

  try {
    const worldUuid = getWorldUuid();
    const payload = Payload.fromPlayers(players, deadPlayers, worldUuid);

    const request = new HttpRequest(`${bvc_server}/api/position`);
    request.setBody(payload.toJSONString());
    request.setMethod(HttpRequestMethod.Post);
    request.setHeaders([
      new HttpHeader('Content-Type', 'application/json'),
      new HttpHeader('X-MC-Access-Token', access_token),
      new HttpHeader('Accept', 'application/json'),
    ]);
    request.setTimeout(REQUEST_TIMEOUT);

    await http
      .request(request)
      .then(() => {
        if (consecutiveFailures >= FAILURE_THRESHOLD) {
          console.info("[BVC] Connection restored");
        }
        consecutiveFailures = 0;
      })
      .catch((error) => {
        consecutiveFailures++;
        if (consecutiveFailures === FAILURE_THRESHOLD) {
          console.warn("[BVC] Backend unreachable, pausing requests");
        }
        if (consecutiveFailures >= FAILURE_THRESHOLD) {
          const backoff = Math.min(
            INITIAL_BACKOFF_MS * Math.pow(2, consecutiveFailures - FAILURE_THRESHOLD),
            MAX_BACKOFF_MS,
          );
          circuitOpenUntil = Date.now() + backoff;
        } else {
          console.warn("[BVC] Failed to send player data:", error);
        }
      });
  } catch (error) {
    console.error("[BVC] Error creating player payload:", error);
  }
}, POLL_INTERVAL);
