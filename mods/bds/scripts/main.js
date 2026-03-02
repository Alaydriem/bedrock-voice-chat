// src/main.ts
import { world, system } from "@minecraft/server";
import {
  HttpHeader,
  HttpRequest,
  HttpRequestMethod,
  http
} from "@minecraft/server-net";
import { variables } from "@minecraft/server-admin";

// src/dto/coordinates.ts
var Coordinates = class _Coordinates {
  constructor(x, y, z) {
    this.x = x;
    this.y = y;
    this.z = z;
  }
  static fromMinecraftLocation(location) {
    return new _Coordinates(
      location.x,
      location.y,
      location.z
    );
  }
  toJSON() {
    return {
      x: this.x,
      y: this.y,
      z: this.z
    };
  }
};

// src/dto/orientation.ts
var Orientation = class _Orientation {
  constructor(x, y) {
    this.x = x;
    this.y = y;
  }
  static fromMinecraftRotation(rotation) {
    return new _Orientation(
      rotation.x,
      rotation.y
    );
  }
  toJSON() {
    return {
      x: this.x,
      y: this.y
    };
  }
};

// src/dto/player.ts
import { GameMode } from "@minecraft/server";
var Player = class _Player {
  constructor(name, dimension, coordinates, deafen, orientation, spectator = false, world_uuid = void 0) {
    this.name = name;
    this.dimension = dimension;
    this.coordinates = coordinates;
    this.deafen = deafen;
    this.orientation = orientation;
    this.spectator = spectator;
    this.world_uuid = world_uuid;
  }
  static fromMinecraftPlayer(player, worldUuid) {
    return new _Player(
      player.name,
      player.dimension.id.replace("minecraft:", ""),
      Coordinates.fromMinecraftLocation(player.location),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      player.getGameMode() === GameMode.Spectator,
      worldUuid
    );
  }
  /**
   * Create a player DTO with death dimension override.
   * Dead players are placed at origin (0,0,0) in the "death" dimension.
   */
  static fromMinecraftPlayerDead(player, worldUuid) {
    return new _Player(
      player.name,
      "death" /* DEATH */,
      new Coordinates(0, 0, 0),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      false,
      worldUuid
    );
  }
  toJSON() {
    return {
      name: this.name,
      dimension: this.dimension,
      coordinates: this.coordinates.toJSON(),
      deafen: this.deafen,
      orientation: this.orientation.toJSON(),
      spectator: this.spectator,
      ...this.world_uuid && { world_uuid: this.world_uuid }
    };
  }
};

// src/dto/payload.ts
var Payload = class _Payload {
  constructor(game = "minecraft", players) {
    this.game = game;
    this.players = players;
  }
  /**
   * Create a payload from Minecraft players.
   * @param players Array of Minecraft players
   * @param deadPlayers Set of player IDs who are currently dead
   * @param worldUuid Optional world UUID for multi-world isolation
   */
  static fromPlayers(players, deadPlayers2 = /* @__PURE__ */ new Set(), worldUuid) {
    const playerDtos = players.map(
      (p) => deadPlayers2.has(p.id) ? Player.fromMinecraftPlayerDead(p, worldUuid) : Player.fromMinecraftPlayer(p, worldUuid)
    );
    return new _Payload("minecraft", playerDtos);
  }
  toJSON() {
    return {
      game: this.game,
      players: this.players.map((p) => p.toJSON())
    };
  }
  toJSONString() {
    return JSON.stringify(this.toJSON());
  }
};

// src/main.ts
var bvc_server = variables.get("bvc_server");
var access_token = variables.get("bvc_access_token");
var debug = variables.get("bvc_debug");
var POLL_INTERVAL = 5;
var MIN_PLAYERS = 2;
var REQUEST_TIMEOUT = 1;
var FAILURE_THRESHOLD = 3;
var INITIAL_BACKOFF_MS = 1e4;
var MAX_BACKOFF_MS = 3e4;
var consecutiveFailures = 0;
var circuitOpenUntil = 0;
var deadPlayers = /* @__PURE__ */ new Set();
var cachedWorldUuid;
function getWorldUuid() {
  if (cachedWorldUuid) {
    return cachedWorldUuid;
  }
  const existing = world.getDynamicProperty("bvc:world_uuid");
  if (typeof existing === "string") {
    cachedWorldUuid = existing;
    console.info("[BVC] Loaded world UUID: " + existing);
    return existing;
  }
  const uuid = [
    randomHex(8),
    randomHex(4),
    "4" + randomHex(3),
    (Math.floor(Math.random() * 4) + 8).toString(16) + randomHex(3),
    randomHex(12)
  ].join("-");
  world.setDynamicProperty("bvc:world_uuid", uuid);
  cachedWorldUuid = uuid;
  console.info("[BVC] Generated world UUID: " + uuid);
  return uuid;
}
function randomHex(length) {
  let result = "";
  for (let i = 0; i < length; i++) {
    result += Math.floor(Math.random() * 16).toString(16);
  }
  return result;
}
console.info("[BVC] Connecting to: " + bvc_server);
world.afterEvents.entityDie.subscribe(
  (event) => {
    const deadEntity = event.deadEntity;
    if (deadEntity.typeId === "minecraft:player") {
      deadPlayers.add(deadEntity.id);
    }
  },
  { entityTypes: ["minecraft:player"] }
);
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
      new HttpHeader("Content-Type", "application/json"),
      new HttpHeader("X-MC-Access-Token", access_token),
      new HttpHeader("Accept", "application/json")
    ]);
    request.setTimeout(REQUEST_TIMEOUT);
    await http.request(request).then(() => {
      if (consecutiveFailures >= FAILURE_THRESHOLD) {
        console.info("[BVC] Connection restored");
      }
      consecutiveFailures = 0;
    }).catch((error) => {
      consecutiveFailures++;
      if (consecutiveFailures === FAILURE_THRESHOLD) {
        console.warn("[BVC] Backend unreachable, pausing requests");
      }
      if (consecutiveFailures >= FAILURE_THRESHOLD) {
        const backoff = Math.min(
          INITIAL_BACKOFF_MS * Math.pow(2, consecutiveFailures - FAILURE_THRESHOLD),
          MAX_BACKOFF_MS
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
