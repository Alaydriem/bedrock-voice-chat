import { world, system } from '@minecraft/server';
import { variables } from '@minecraft/server-admin';
import { Payload, Player } from './dto';
import { AudioPlayerManager } from './audio/player_manager';
import { AudioComponentRegistry } from './audio/components';
import { DiscCommand } from './commands/mod';
import { httpClient } from './net';

const bvc_server: string = variables.get('bvc_server');
const access_token: string = variables.get('bvc_access_token');
const minimum_players_raw = variables.get("bvc_minimum_players");
const minimum_players: number = typeof minimum_players_raw === 'number' && minimum_players_raw >= 1
  ? Math.floor(minimum_players_raw)
  : 1;

const POLL_INTERVAL = 5;
const REQUEST_TIMEOUT = 1;

const FAILURE_THRESHOLD = 3;
const INITIAL_BACKOFF_MS = 10_000;
const MAX_BACKOFF_MS = 30_000;
let consecutiveFailures = 0;
let circuitOpenUntil = 0;

const deadPlayers = new Set<string>();

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

const audioManager = new AudioPlayerManager(bvc_server, access_token);
const componentRegistry = new AudioComponentRegistry(audioManager, getWorldUuid);
componentRegistry.register();

DiscCommand.register();

httpClient.ensureLoaded().then((available) => {
  if (!available) {
    console.warn("[BVC] HTTP unavailable; position polling and disc events will not be sent");
    return;
  }

  console.info("[BVC] Connecting to: " + bvc_server);

  world.afterEvents.entityDie.subscribe(
    (event) => {
      const deadEntity = event.deadEntity;
      if (deadEntity.typeId === 'minecraft:player') {
        deadPlayers.add(deadEntity.id);
      }
    },
    { entityTypes: ['minecraft:player'] }
  );

  world.afterEvents.playerSpawn.subscribe((event) => {
    deadPlayers.delete(event.player.id);
  });

  world.afterEvents.playerLeave.subscribe((event) => {
    deadPlayers.delete(event.playerId);

    system.runTimeout(async () => {
      try {
        const worldUuid = getWorldUuid();
        const phantom = Player.fromDisconnectedPlayer(event.playerName, worldUuid);
        const payload = new Payload('minecraft', [phantom]);

        await httpClient.request(
          `${bvc_server}/api/position`,
          'Post',
          payload.toJSONString(),
          [
            ['Content-Type', 'application/json'],
            ['X-MC-Access-Token', access_token],
            ['Accept', 'application/json'],
          ],
          REQUEST_TIMEOUT
        );
      } catch (error) {
        console.error("[BVC] Error sending disconnect phantom:", error);
      }
    }, 5);
  });

  system.runInterval(async () => {
    const players = world.getAllPlayers();

    if (players.length < minimum_players) {
      return;
    }

    const now = Date.now();
    if (consecutiveFailures >= FAILURE_THRESHOLD && now < circuitOpenUntil) {
      return;
    }

    try {
      const worldUuid = getWorldUuid();
      const payload = Payload.fromPlayers(players, deadPlayers, worldUuid);

      const response = await httpClient.request(
        `${bvc_server}/api/position`,
        'Post',
        payload.toJSONString(),
        [
          ['Content-Type', 'application/json'],
          ['X-MC-Access-Token', access_token],
          ['Accept', 'application/json'],
        ],
        REQUEST_TIMEOUT
      );

      if (response && response.status >= 200 && response.status < 300) {
        if (consecutiveFailures >= FAILURE_THRESHOLD) {
          console.info("[BVC] Connection restored");
        }
        consecutiveFailures = 0;
      } else {
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
        }
      }
    } catch (error) {
      console.error("[BVC] Error creating player payload:", error);
    }
  }, POLL_INTERVAL);
});
