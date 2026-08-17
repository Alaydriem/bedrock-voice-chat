---
title: Troubleshooting
description: Diagnosing a BVC deployment from the operator side.
sidebar:
  label: Troubleshooting
  order: 11
---

Work through a Bedrock Voice Chat deployment from the outside in. Most reports resolve in the first two checks.

## 1. Is the server reachable?

```bash
curl https://example.bedrockvc.stream/api/config
```

JSON back means the process is up, TLS is valid, and TCP/443 is open.

Anything else and the problem is here, not in the Addon or the client.

| Symptom | Cause |
|---|---|
| Connection refused | Not running, or the port is not published. |
| Timeout | Firewall or port forwarding. |
| Certificate error | See [TLS](/wiki/server/tls/). Self-signed will not work. |
| 404 | Something else is answering on that address. |

## 2. Is UDP open?

This is the one that gets missed. HTTP and voice use the same port number over **different protocols**. TCP/443 can be open while UDP/443 is closed. Sign-in works, nobody hears anything, and every HTTP check reports green.

Verify UDP/443 is published and reachable, separately from TCP. In Docker that means `-p 443:443/udp` as well as `/tcp`.

If clients are on networks that block QUIC, publish the redundant port on 8443/UDP too.

## 3. Is the Addon relaying?

```bash
curl -H "Authorization: Bearer <your token>" https://example.bedrockvc.stream/api/position
```

Populated means the whole chain works. Empty means the game server is not sending.

For BDS, set `content-log-console-output-enabled=true` and look for `[BVC] Connecting to:` on world load. No line means the Addon is not running. Check Beta APIs, the world's pack list, and `permissions.json`.

The most common cause of an empty response is an `access_token` mismatch between `config.hcl` and the Addon. It is rejected silently on every update.

## 4. Are the players whitelisted?

```bash
bvc whoami
bvc permission list -p <player> -g minecraft
```

BVC is deny-by-default. See [players and permissions](/wiki/server/players-and-permissions/).

## Specific symptoms

**Sign-in works, no audio at all.** Almost always UDP. Go back to step 2.

**Audio works for some players, not others.** Their network is blocking QUIC. Publish 8443/UDP.

**Everything worked, then stopped after a Minecraft update.** Only affects Realms and Aternos. See [version support](/wiki/platforms/version-support/).

**Console players cannot connect.** The console and the device running BVC must share a network. The session is listed under Worlds, not Servers. See [console and mobile](/wiki/platforms/console-and-mobile/).

**Everyone was signed out at once.** The mTLS CA in `certs_path` was deleted or replaced. Every issued client certificate is now invalid and everyone must sign in again. Restore from backup if you have one.

**Audio degrades as players join.** BVC is CPU-bound. Add cores. Embedded mode on a shared host is a common cause. Move BVC to its own machine.

**A player connects fine, then goes silent after a few minutes, repeatedly.** Usually mobile data or another carrier-grade NAT. The carrier drops the idle UDP mapping. The connection returns on a new source port, which the server counts as a new network path. QUIC allows a small fixed number per connection and never reclaims them. The connection survives the first few rebindings, then stops carrying audio without closing.

The server sends a 10-second keepalive, under the shortest carrier timeouts measured so far. No operator action should be needed. If you still see this, collect the player's network type and open an issue. The keepalive interval is not configurable.

## Turning up the detail

```hcl
log {
    level = "debug"
}
```

Logs QUIC path and handshake detail, which explains connections that establish and then fail. Loud. Turn it back down afterwards.

## Reporting it

Include the output of `/api/config`, whether UDP/443 is open, whether `/api/position` returns data, your deployment shape (Docker, bare metal, embedded), and what you already ruled out.

[Discord](https://discord.gg/CdtchD5zxr) or a [GitHub issue](https://github.com/Alaydriem/bedrock-voice-chat/issues).
