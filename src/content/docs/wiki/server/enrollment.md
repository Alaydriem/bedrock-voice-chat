---
title: Assigned hostnames
description: Getting a hostname and a certificate for a server that has no domain.
sidebar:
  label: Assigned hostnames
  order: 4
---

:::caution[Work in progress. Limited availability.]
Assigned hostnames are in development and are **not in a release yet**. The configuration can change.

Only **some Patreon supporters and YouTube members** can use this feature. A membership alone does not qualify. Only some membership tiers qualify, and these tiers can change during development.
:::

A Bedrock Voice Chat server needs a hostname and a valid certificate. The usual method is to buy a domain, point its DNS at your host, and configure an ACME provider.

Enrollment gives you a hostname without a domain. Bedrock Voice Chat gives you a name under `bedrockvc.stream` and renews a certificate for it. You do not need a domain, DNS settings, or an ACME provider.

You get one name. BVC chooses the name. The name stays with your server while your membership is active and the server keeps its key.

## Requirements

You need a qualifying Patreon or YouTube membership on the Discord account that you sign in with. Patreon and YouTube memberships sync to a Discord role, and BVC checks for that role. A membership that has not synced yet does not qualify.

## Enroll

1. Open the enrollment link that eligible members receive, and sign in with Discord. You get an enrollment token.

2. Put the token in `config.hcl`:

   ```hcl
   server {
       enrollment {
           token = "bvcenroll..."
       }
   }
   ```

3. Start the server. The log shows the name that you got:

   ```
   this server is reachable at its assigned name  url=https://cobblestone-thicket-savanna.bedrockvc.stream
   ```

Do not add a `tls` block. The server adds the assigned name to its certificate and manages the certificate itself. If you add a `tls` block, the server does not start. See [Conflicts](#conflicts).

The server uses the token one time, at first start. After that, the server identifies itself with a key that it makes. You can keep the token in the file or delete it.

:::caution[Back up the server key]
Your server makes a key at first start. The name is linked to that key. If you lose the key, you must enroll again and you get a **different** name. Back up the server's data directory. Keep the directory on a volume that is kept when the container is replaced.
:::

## Address

Set `address` to the IP address that clients connect to. If you do not set it, BVC does not publish a DNS record for your name. Clients then cannot find the server, even though the certificate is valid.

If the server has a name and no address, it writes a warning at startup.

```hcl
server {
    enrollment {
        token   = "bvcenroll..."
        address = "203.0.113.10"
    }
}
```

BVC does not detect the address. Behind NAT, behind CGNAT, or on a LAN, the address that outside servers see is not the address that clients use. A detected address is correct only for a server with port forwarding.

BVC publishes a private address (`192.168.x.x`, `10.x.x.x`). The name then resolves only on the network that set it. BVC checks a public address every day.

## Conflicts

You cannot use an assigned name and your own certificate configuration together. If you set both, the server does not start. The error names the two settings:

```
server.enrollment.token and tls.acme are mutually exclusive; remove one
```

Use one of these:

| Setting | When |
|---|---|
| `server.enrollment` | You have no domain. |
| `tls.acme` | You have a domain and a supported DNS provider. See [TLS certificates](/wiki/server/tls/). |
| `tls.certificate` / `tls.key` | You already have a certificate. |

## Errors

| Symptom | Cause |
|---|---|
| `not entitled` | The Discord account that you signed in with does not have a qualifying role. Make sure that your Patreon or YouTube membership has synced. |
| `already registered` | That Discord account already has a name. Each member gets one name. |
| `unknown token` or `already redeemed` | The token was used, typed incorrectly, or is more than one day old. Sign in again to get a new token. |
| A different name after a rebuild | The server's key was lost. Restore the data directory, or use the new name. |
| The name does not resolve | No `address` is set, and BVC did not publish a DNS record. Set `address` and restart the server. |
