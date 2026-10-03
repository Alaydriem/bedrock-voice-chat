---
title: Server WebSocket
description: The BVC server's position feed and chat bridge, how to connect to each, and their limits.
sidebar:
  label: Server WebSocket
  order: 6
---

The BVC server has two WebSocket feeds. Both use `wss://` on the same host and port as the [HTTP API](/wiki/reference/http-api/).

| Feed | Address | Used by |
|---|---|---|
| Position feed | `/api/websocket/positions` | The BVC app, to show who is near the player |
| Chat bridge | `/api/websocket/chat` | The BVC Addon and mods, to pass chat between the game and the app |

## Reference

**[bedrockvoicechat.com/websocket/server](https://www.bedrockvoicechat.com/websocket/server)**

Every field of every message is listed there.

## Position feed

The position feed tells a player who is near them in game, and where. The server sends a new snapshot every 500 ms. Each snapshot is complete and replaces the previous one.

### Connect

1. Call `POST /api/websocket/ticket` with the player's BVC client certificate. The response contains a `ticket` and `expires_in`.
2. Within 60 seconds, open the socket. Offer two subprotocols: `ticket.<ticket>` and `bvc.positions.v1`.
3. Read the messages. Each one is a JSON snapshot.

```js
const socket = new WebSocket(
  'wss://voice.example.com/api/websocket/positions',
  [`ticket.${ticket}`, 'bvc.positions.v1'],
);

socket.onmessage = (message) => {
  const snapshot = JSON.parse(message.data);
  for (const player of snapshot.positions) {
    console.log(player.name, player.distance, player.bearing_deg);
  }
};
```

A missing, expired or used ticket gets HTTP 401 and no socket.

### Snapshot

```json
{
  "seq": 42,
  "positions": [
    { "name": "minecraft:Steve", "presence": "voice", "bearing_deg": 90, "distance": 12, "elevation": -3 }
  ]
}
```

| Field | Meaning |
|---|---|
| `seq` | Counts up by one for each snapshot. Ignore a snapshot with a lower number than the last one you used. |
| `name` | The player, as `game:gamertag`. |
| `presence` | `voice` if they are connected to voice chat. `game` if they are in the game without BVC. |
| `bearing_deg` | Direction in degrees, clockwise from the way you face. 0 is ahead, 90 is right, 180 is behind, 270 is left. |
| `distance` | Distance in blocks, measured flat along the ground. |
| `elevation` | Height difference in blocks. Positive is above you. |

### Who appears

- Every player within voice range. Voice range is [`voice.spatial_audio.broadcast_range`](/wiki/reference/configuration/#voicespatial_audio), 48 blocks by default.
- Up to 16 more players beyond voice range, nearest first. The feed reaches 2.5 times voice range, and never more than 256 blocks.
- Only players in the same world and dimension as you.
- Players without BVC, with `presence` set to `game`.

The feed never sends coordinates. Every position is relative to you.

### Limitations

- A ticket needs the player's client certificate. The BVC app receives it when the player signs in. A server operator or a third-party tool cannot get one.
- Each ticket works once. Asking for a new ticket cancels the previous one.
- The server closes the socket after 6 hours. Get a new ticket and connect again.
- A player who is not in the game gets snapshots with an empty `positions` list. The socket stays open.
- Messages sent to the server on this socket are ignored.

## Chat bridge

The chat bridge carries chat between a game server and the BVC app. The BVC Addon and the BVC mods use it. Use it to connect chat from your own plugin or mod.

### Connect

1. Open the socket with the header `Authorization: Bearer <token>`. The token is [`server.minecraft.access_token`](/wiki/reference/configuration/#serverminecraft).
2. Do not offer a subprotocol. The handshake fails if you do.
3. Send `hello` first.
4. Send `chat` and `event` as things happen in game. Show each `say` you receive in game chat.

```js
import WebSocket from 'ws';

const socket = new WebSocket('wss://voice.example.com/api/websocket/chat', {
  headers: { Authorization: `Bearer ${process.env.BVC_ACCESS_TOKEN}` },
});

socket.on('open', () => {
  socket.send(JSON.stringify({ t: 'hello', world: worldId, world_name: 'Survival', game: 'minecraft' }));
  socket.send(JSON.stringify({ t: 'chat', author: 'Steve', text: 'hello from the game' }));
});

socket.on('message', (data) => {
  const message = JSON.parse(data);
  if (message.t === 'say') {
    broadcastInGame(`<${message.author}> ${message.text}`);
  }
});
```

### Messages

Every message is a JSON object. The `t` field says which kind it is.

| `t` | Direction | Fields | Meaning |
|---|---|---|---|
| `hello` | Game to server | `world`, `world_name`, `game`, `worlds` | Names the world. Send it first, once. |
| `chat` | Game to server | `author`, `text` | A player typed a message in game. |
| `event` | Game to server | `text` | A line from the game itself, such as a death or a join. Shown without an author. |
| `say` | Server to game | `author`, `text` | A player typed a message in the app. |

`world` is the world ID your Addon or mod reports player positions with. `game` is `minecraft`. `worlds` is optional. It lists extra world IDs that share this chat, for a server that gives each dimension its own ID.

### Limitations

- Use one socket for each world. A new socket for the same world closes the old one.
- A `chat` or `event` sent before `hello` closes the socket. A second `hello` is ignored.
- With [`server.features.chat`](/wiki/reference/configuration/#serverfeatures) set to `false`, the socket connects but no chat goes through in either direction.
- Chat is not stored. Messages sent while the socket is closed are lost.
- Read `say` messages promptly. The server holds up to 64 unread messages and drops the rest.
