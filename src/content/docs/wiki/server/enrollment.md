---
title: Assigned hostnames
description: Getting a hostname and a certificate for a server that has no domain.
sidebar:
  label: Assigned hostnames
  order: 4
---

:::caution[Work in progress — limited availability]
Assigned hostnames are under active development and are **not in a release yet**. The configuration shape may still change.

Availability is limited to **select Patreon supporters and YouTube members**. Holding a membership does not by itself qualify — eligibility is granted per tier, and the qualifying tiers may change while this is in development.
:::

A Bedrock Voice Chat server needs a hostname and a valid certificate. Buying a domain, pointing DNS at your host and configuring an ACME provider is the ordinary way to get both, and it is the step most people give up on.

Enrollment is the other way. Bedrock Voice Chat gives you a name under `bedrockvc.stream` and keeps a certificate for it renewed. You never own a domain, never touch DNS, and never configure ACME.

You get one name, it is chosen for you, and it stays with your server for as long as your membership does.

## What you need

A qualifying Patreon or YouTube membership, held on the Discord account you sign in with. Each syncs into a Discord role, and the role is what the check reads — a membership that has not synced yet does not qualify.

## Enrol

1. Sign in with Discord, from the enrollment link shared with eligible members. You are handed an enrollment token.

2. Put it in `config.hcl`:

   ```hcl
   server {
       enrollment {
           token = "bvcenroll..."
       }
   }
   ```

3. Start the server. It logs the name you were given:

   ```
   this server is reachable at its assigned name  url=https://cobblestone-thicket-savanna.bedrockvc.stream
   ```

That is the whole configuration. Do not write a `tls` block — the server appends the assigned name to its own SANs and configures its own certificate. Writing one is an error, not an override; see [conflicts](#conflicts).

The token is spent on first boot. Afterwards the server proves who it is with a key it generated itself, so the token can stay in the file or be deleted, whichever you prefer.

:::caution[The name follows the key, not the config file]
Your server generates a key on first start and that key is what owns the name. Losing it means enrolling again and being given a **different** name. Back up the server's data directory, and keep it on a volume that survives a container being replaced.
:::

## Publishing an address

You have to declare the address. Without it the assigned name has no DNS record, and a name that resolves to nothing is a name nobody can connect to — your certificate is valid and every client still fails in DNS.

The server warns at startup when it has a name and no address.

```hcl
server {
    enrollment {
        token   = "bvcenroll..."
        address = "203.0.113.10"
    }
}
```

Declared rather than detected. Behind NAT, behind CGNAT, or on a LAN, no address observed from the outside would be the right one, and a guess is correct for a port-forwarded deployment and silently wrong for every other.

A private address (`192.168.x.x`, `10.x.x.x`) is published and resolves only on the network that declared it. A public address is re-checked daily against the server that answers there.

## Conflicts

An assigned name and a certificate you configure yourself are mutually exclusive. The server refuses to start with both, naming which pair collided:

```
server.enrollment.token and tls.acme are mutually exclusive; remove one
```

Pick one of the three:

| | When |
|---|---|
| `server.enrollment` | You have no domain. |
| `tls.acme` | You have a domain and a supported DNS provider. See [TLS certificates](/wiki/server/tls/). |
| `tls.certificate` / `tls.key` | You already hold a certificate. |

## When it fails

| Symptom | Cause |
|---|---|
| `not entitled` | The Discord account you signed in with holds no qualifying role. Check that your Patreon or YouTube membership has synced. |
| `already registered` | That Discord account already has a name. One is issued per member. |
| `unknown token` or `already redeemed` | The token was used, mistyped, or older than a day. Sign in again for another. |
| A different name after a rebuild | The server's key was lost. Restore the data directory, or accept the new name. |
| The name does not resolve | No `address` was declared, so no A record was published. Set it and restart. |
