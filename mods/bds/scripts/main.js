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
  constructor(name, dimension, coordinates, deafen, orientation, spectator = false) {
    this.name = name;
    this.dimension = dimension;
    this.coordinates = coordinates;
    this.deafen = deafen;
    this.orientation = orientation;
    this.spectator = spectator;
  }
  static fromMinecraftPlayer(player) {
    return new _Player(
      player.name,
      player.dimension.id.replace("minecraft:", ""),
      Coordinates.fromMinecraftLocation(player.location),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      player.getGameMode() === GameMode.Spectator
    );
  }
  /**
   * Create a player DTO with death dimension override.
   * Dead players are placed at origin (0,0,0) in the "death" dimension.
   */
  static fromMinecraftPlayerDead(player) {
    return new _Player(
      player.name,
      "death" /* DEATH */,
      new Coordinates(0, 0, 0),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      false
    );
  }
  toJSON() {
    return {
      name: this.name,
      dimension: this.dimension,
      coordinates: this.coordinates.toJSON(),
      deafen: this.deafen,
      orientation: this.orientation.toJSON(),
      spectator: this.spectator
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
   */
  static fromPlayers(players, deadPlayers2 = /* @__PURE__ */ new Set()) {
    const playerDtos = players.map(
      (p) => deadPlayers2.has(p.id) ? Player.fromMinecraftPlayerDead(p) : Player.fromMinecraftPlayer(p)
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
var deadPlayers = /* @__PURE__ */ new Set();
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
  try {
    const payload = Payload.fromPlayers(players, deadPlayers);
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
    }).catch((error) => {
      console.warn("Failed to send player data:", error);
    });
  } catch (error) {
    console.error("Error creating player payload:", error);
  }
}, POLL_INTERVAL);
