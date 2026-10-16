---
title: "Troubleshooting"
description: "Diagnosing a BVC deployment from the operator side."
source: https://www.bedrockvoicechat.com/wiki/server/troubleshooting/
---
Check a Bedrock Voice Chat deployment from the outside in. Do the checks in order.

## 1. Server reachability

```bash
curl https://example.bedrockvc.stream/api/config
```

If the response is JSON, the process is running, TLS is valid, and TCP/443 is open.

If the response is not JSON, the problem is in the server, its network, or its certificate.

| Symptom | Cause |
|---|---|
| Connection refused | The server is not running, or the port is not published. |
| Timeout | Firewall or port forwarding. |
| Certificate error | See [TLS](/wiki/server/tls/). A self-signed certificate does not work. |
| 404 | A different service is listening on that address. |

## 2. UDP port

HTTP uses TCP/443. Voice uses QUIC on UDP/28280. If TCP/443 is open and UDP/28280 is closed, sign-in works and HTTP checks pass, but nobody hears audio.

Make sure that UDP/28280 is published and reachable. Test it separately from TCP. In Docker, publish `-p 28280:28280/udp`.

When a client's network blocks UDP, the client sends voice over WebSocket on TCP/443.

## 3. Addon position updates

```bash
curl -H "Authorization: Bearer <your token>" https://example.bedrockvc.stream/api/position
```

If the response lists players, the game server, the Addon and BVC all work. If the list is empty, the game server does not send positions.

For BDS, set `content-log-console-output-enabled=true`. When the world loads, look for `[BVC] Connecting to:`. If the line is missing, the Addon is not running. Check Beta APIs, the world's pack list, and `permissions.json`.

The most common cause of an empty list is an `access_token` in the Addon that does not match `config.hcl`. BVC rejects each of these position updates without an error.

## 4. Whitelist

```bash
bvc whoami
bvc permission list -p <player> -g minecraft
```

A player can sign in only after they join your game server once, or after an admin adds them. See [players and permissions](/wiki/server/players-and-permissions/).

## Specific symptoms

**Sign-in works, but there is no audio.** The cause is usually a closed UDP port. Go back to step 2.

**Audio works for some players and not for others.** The networks of those players probably block UDP. Make sure that TCP/443 is open for the WebSocket fallback.

**Everything worked, then stopped after a Minecraft update.** This affects only Realms and Aternos. See [version support](/wiki/platforms/version-support/).

**Console players cannot connect.** The console and the device that runs BVC must be on the same network. In Minecraft, the session shows on the Worlds tab, not the Servers tab. See [console and mobile](/wiki/platforms/console-and-mobile/).

**All players were signed out at the same time.** The database that holds the mTLS certificate authority was lost or replaced. All client certificates are now invalid, and every player must sign in again. If you have a backup of the database, restore it.

**Audio gets worse as more players join.** The server needs more CPU. Add cores. Embedded mode on a shared host often causes this. Move BVC to its own machine.

**A player connects, then goes silent after a few minutes, again and again.** This usually happens on mobile data or behind carrier-grade NAT. The carrier closes the idle UDP mapping, and the player's device reconnects from a new port. QUIC permits only a few port changes for each connection. After that limit, the connection stays open but sends no audio.

The server sends a keepalive every 10 seconds. This is shorter than the carrier timeouts that are known. You cannot change the interval. If the problem continues, record the player's network type and open an issue.

## Debug logging

```hcl
log {
    level = "debug"
}
```

At `debug`, the server logs QUIC path and handshake details. Use it when connections open and then fail. The server writes many lines at this level. Set the level back to `info` when you are done.

## Reporting a problem

Include:

- The output of `/api/config`.
- Whether UDP/28280 is open.
- Whether `/api/position` returns data.
- How you run the server (Docker, bare metal, or embedded).
- What you already checked.

Report on [Discord](https://discord.gg/CdtchD5zxr) or open a [GitHub issue](https://github.com/Alaydriem/bedrock-voice-chat/issues).