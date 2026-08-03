---
title: Enabling Beta APIs
description: Turning on the Beta APIs experiment for a Bedrock server world.
sidebar:
  label: Enabling Beta APIs
  order: 7
---

The BVC Addon uses beta script modules, so the world it runs in must have the **Beta APIs** experiment enabled. Without it the Addon loads and does nothing — no error, no console line, no positions.

This is the single most common reason a correct-looking Bedrock install produces silence.

:::caution[There is no server.properties setting for this]
Experiments are stored per-world in `level.dat`, not in server configuration. You cannot enable them by editing a config file on the server, and a fresh world created by BDS on first boot does not have them.
:::

## The reliable way: build the world in the client

Enable the experiment where there is a UI for it, then move the world to the server.

1. In the Minecraft Bedrock **client**, create a new world.
2. Under **Settings → Experiments**, turn on **Beta APIs**.
3. Add the BVC Addon to the world's behavior and resource packs while you are here.
4. Create the world, then leave it immediately.
5. **Settings → Export World**, and save the `.mcworld`.
6. Put it on your server:
   - **BDS** — rename to `.zip`, extract into `worlds/<your world name>/`, and set `level-name` in `server.properties` to match.
   - **A host with a panel** — upload the `.mcworld` or `.zip` through their world upload.

This is the same sequence the [no-net Addon](/wiki/server/nonet-addon/) guide walks through, because Realms and Aternos need it for the same reason.

## Existing worlds

You cannot turn the experiment on for a world that is already running by changing anything on the server. The flag lives in that world's `level.dat`.

Three options, in order of how much they can go wrong:

**Open it in the client.** Copy the world off the server, open it in the Bedrock client, enable Beta APIs under Experiments, exit, and copy it back. Straightforward, and it uses the supported UI.

**Use your host's panel.** Aternos and some other hosts expose experiment toggles directly. If yours does, use it.

**Edit `level.dat` with an NBT editor.** The experiment flags live in an `experiments` compound. The one you want corresponds to the Beta APIs toggle, and the world also has to be marked as having used experiments at all.

:::caution
The exact NBT key names have changed across Bedrock versions. Rather than trusting a key name from any guide including this one, create a throwaway world in the client with Beta APIs enabled, open both `level.dat` files side by side, and copy whatever actually differs.
:::

Back up the world first. A malformed `level.dat` will not load.

## What enabling it costs

**Achievements are permanently disabled** for that world. This is Mojang's behaviour for any experiment, not something BVC controls.

**The world is permanently marked as having used experiments.** Turning the experiment back off later does not clear the marking.

**Experimental features can change between Minecraft versions.** That is what "experimental" means, and it is part of why BVC's Bedrock support tracks releases rather than leading them.

## Checking it worked

Set this in `server.properties`:

```properties
content-log-console-output-enabled=true
```

On world load you should see:

```
[BVC] Connecting to: <your server>
```

No line means the Addon is not running. In order of likelihood:

1. Beta APIs are not actually enabled on **the world the server is loading** — check `level-name` matches the world you edited.
2. The Addon is not in the world's pack list.
3. `config/default/permissions.json` is missing a module — see [Bedrock Addon](/wiki/server/bedrock-addon/).
4. `bvc_server` is unset in `variables.json`.

## Java servers

Not applicable. Experiments are a Bedrock concept, and the Fabric and Paper mods have no equivalent requirement. See [Java mod](/wiki/server/java-mod/).
