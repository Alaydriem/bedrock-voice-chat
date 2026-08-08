import { world, system } from '@minecraft/server';
import { Payload, Player } from './dto';
import { AudioPlayerManager } from './audio/player_manager';
import { AudioComponentRegistry } from './audio/components';
import { ChatEjectListener } from './audio/chat_eject_listener';
import { ChatPresenceListener } from './audio/chat_presence_listener';
import { BvcsListener } from './state/bvcs_listener';
import { StateCacheStore } from './state/cache_store';
import { NetStateSource, NoNetStateSource } from './state/state_source';
import type { PanelFeed } from './state/state_source';
import { ControlPanel } from './ui/panel';
import { PanelTestConfig } from './ui/panel_test';
import { DiscCommand, JukeboxTestCommands } from './commands/mod';
import { ControlCommands } from './commands/control';
import { NetAudioSender, NoNetAudioSender } from './audio/sender';
import { NetControlSender, NoNetControlSender } from './control/sender';
import type { ControlSender } from './control/sender';
import { httpClient } from './net';
import { serverAdminConfig } from './config';
import { ChatChannel, ChatListener } from './chat';

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
    console.info('[BVC] Loaded world UUID: ' + existing);
    return existing;
  }

  const uuid = [
    randomHex(8),
    randomHex(4),
    '4' + randomHex(3),
    (Math.floor(Math.random() * 4) + 8).toString(16) + randomHex(3),
    randomHex(12),
  ].join('-');

  world.setDynamicProperty('bvc:world_uuid', uuid);
  cachedWorldUuid = uuid;
  console.info('[BVC] Generated world UUID: ' + uuid);
  return uuid;
}

function randomHex(length: number): string {
  let result = '';
  for (let i = 0; i < length; i++) {
    result += Math.floor(Math.random() * 16).toString(16);
  }
  return result;
}

const audioManager = new AudioPlayerManager();
const componentRegistry = new AudioComponentRegistry(
  audioManager,
  getWorldUuid,
);
componentRegistry.register();

const jukeboxTestCommands = new JukeboxTestCommands(audioManager, getWorldUuid);
jukeboxTestCommands.register();

const chatEjectListener = new ChatEjectListener(audioManager, getWorldUuid);
chatEjectListener.register();

const chatPresenceListener = new ChatPresenceListener();
chatPresenceListener.register();

// Net-mode chat relay. Only starts where the network module exists; a no-net world has the
// desktop proxy observing chat locally instead, and this would have nothing to talk to.
void (async () => {
  const hasNet = await httpClient.ensureLoaded();
  if (!hasNet || !serverAdminConfig.bvcServer) {
    return;
  }

  let chatListener: ChatListener | null = null;
  const chatChannel = new ChatChannel(
    serverAdminConfig.bvcServer,
    serverAdminConfig.accessToken,
    getWorldUuid(),
    serverAdminConfig.worldName,
    (author, text) => chatListener?.say(author, text),
  );
  chatListener = new ChatListener(chatChannel);
  chatListener.register();
  chatChannel.start();
})();

// Panel state: the !bvcs: reverse ride feeds per-player caches in no-net mode;
// the control panel binds to the same store.
const stateCacheStore = new StateCacheStore();
const bvcsListener = new BvcsListener((name) => stateCacheStore.for(name));
bvcsListener.register();

// A leave must tear down that player's panel plumbing even if their form never
// settled: stop any net poll and drop the stale cache entry.
world.afterEvents.playerLeave.subscribe((event) => {
  panelFeed?.stop(event.playerName);
  stateCacheStore.evict(event.playerName);
});

DiscCommand.register();

// Resolved once the sender selection below runs; the commands read it lazily.
let controlSender: ControlSender | null = null;
let panelFeed: PanelFeed | null = null;
const panelTestConfig = new PanelTestConfig();
const controlPanel = new ControlPanel(
  () => controlSender,
  stateCacheStore,
  () => panelFeed,
  panelTestConfig,
);
ControlCommands.register(
  () => controlSender,
  (player) => controlPanel.open(player),
  panelTestConfig,
);

serverAdminConfig
  .ensureLoaded()
  .then(() => httpClient.ensureLoaded())
  .then((available) => {
    if (!available) {
      audioManager.setSender(new NoNetAudioSender());
      const noNetSender = new NoNetControlSender();
      controlSender = noNetSender;
      panelFeed = new NoNetStateSource(() => noNetSender);
      console.warn(
        '[BVC] HTTP unavailable; using no-net jukebox bus (position polling and HTTP disc events disabled)',
      );
      return;
    }

    controlSender = new NetControlSender(serverAdminConfig);
    panelFeed = new NetStateSource(serverAdminConfig);

    audioManager.setSender(
      new NetAudioSender(
        serverAdminConfig,
        (key) => audioManager.forceEject(key),
        (state) => audioManager.locationKey(getWorldUuid(), state.coordinates),
      ),
    );

    const bvcServer = serverAdminConfig.bvcServer;
    const accessToken = serverAdminConfig.accessToken;
    const minimumPlayers = serverAdminConfig.minimumPlayers;

    console.info('[BVC] Connecting to: ' + bvcServer);

    world.afterEvents.entityDie.subscribe(
      (event) => {
        const deadEntity = event.deadEntity;
        if (deadEntity.typeId === 'minecraft:player') {
          deadPlayers.add(deadEntity.id);
        }
      },
      { entityTypes: ['minecraft:player'] },
    );

    world.afterEvents.playerSpawn.subscribe((event) => {
      deadPlayers.delete(event.player.id);
    });

    world.afterEvents.playerLeave.subscribe((event) => {
      deadPlayers.delete(event.playerId);

      system.runTimeout(async () => {
        try {
          const worldUuid = getWorldUuid();
          const phantom = Player.fromDisconnectedPlayer(
            event.playerName,
            worldUuid,
          );
          const payload = new Payload('minecraft', [phantom]);

          await httpClient.request(
            `${bvcServer}/api/position`,
            'Post',
            payload.toJSONString(),
            [
              ['Content-Type', 'application/json'],
              ['X-MC-Access-Token', accessToken],
              ['Accept', 'application/json'],
            ],
            REQUEST_TIMEOUT,
          );
        } catch (error) {
          console.error('[BVC] Error sending disconnect phantom:', error);
        }
      }, 5);
    });

    system.runInterval(async () => {
      const players = world.getAllPlayers();

      if (players.length < minimumPlayers) {
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
          `${bvcServer}/api/position`,
          'Post',
          payload.toJSONString(),
          [
            ['Content-Type', 'application/json'],
            ['X-MC-Access-Token', accessToken],
            ['Accept', 'application/json'],
          ],
          REQUEST_TIMEOUT,
        );

        if (response && response.status >= 200 && response.status < 300) {
          if (consecutiveFailures >= FAILURE_THRESHOLD) {
            console.info('[BVC] Connection restored');
          }
          consecutiveFailures = 0;
        } else {
          consecutiveFailures++;
          if (consecutiveFailures === FAILURE_THRESHOLD) {
            console.warn('[BVC] Backend unreachable, pausing requests');
          }
          if (consecutiveFailures >= FAILURE_THRESHOLD) {
            const backoff = Math.min(
              INITIAL_BACKOFF_MS *
                Math.pow(2, consecutiveFailures - FAILURE_THRESHOLD),
              MAX_BACKOFF_MS,
            );
            circuitOpenUntil = Date.now() + backoff;
          }
        }
      } catch (error) {
        console.error('[BVC] Error creating player payload:', error);
      }
    }, POLL_INTERVAL);
  });
