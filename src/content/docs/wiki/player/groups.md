---
title: Groups
description: Talk to your party at full volume regardless of distance, and keep a group private.
sidebar:
  label: Groups
  order: 4
---

In a group, each member hears all other members at full volume, wherever they are in the world.

<div class="window"><img src="/assets/wiki/dashboard-group.jpg" alt="Group panel in the sidebar on the left, showing members" /></div>

## Create a group

Select **New group** in the sidebar on the left, or type `/bvc:groupcreate` in game. In the app, you join the new group and the group editor opens, so you can change its name.

You own each group that you create. When you leave your group, you still own it, and you can join it again. A [Private Group](#private-groups) is different.

There is no limit on the number of groups.

## Join a group

You can join a group in these ways:

- In the app, select the group name in the sidebar.
- On Bedrock, type `/bvc:panel` and join from the panel.
- In game, type `/bvc:groupjoin <code>`.

:::caution
A group that is not a Private Group is open. Any player on the server can join it at any time.
:::

## Leave or close a group

To leave a group, select the group that you are in, or swipe the group to the left and select **Leave**.

If you own the group, swipe it to the left and select **Close** to close it. The group ends for all members.

## Groups and proximity

In a group, you hear all group members at full volume. You also hear players near you. Their volume changes with distance.

In a Private Group, players near you cannot hear you. You still hear them.

## Private Groups

A Private Group is end-to-end encrypted. The BVC server cannot hear what its members say.

A player cannot join a Private Group alone. A member who is online must admit each new player. The new player and the member both see the same 8-digit code in the BVC app, and both confirm that it matches.

A Private Group has a lock next to its name in the sidebar. On Bedrock, the panel shows it as `[Ask] <name>`.

### Make a group private

1. Select **New group**. The group editor opens.
2. Turn on **Private Group**.
3. Select **Save**.

To change a group that you already own, swipe the group to the left and select **Edit**.

Only the owner can make a group private, and only while the owner is the only member. If another player joins first, the app shows "Someone joined. Turn this on while you are alone."

You cannot make a Private Group open again. To have an open group, create a new group.

### Ask to join a Private Group

1. Select the group in the sidebar. On a phone, tap it.
2. Select **Ask to join**. The app shows **Waiting for a member**.
3. When a member checks your request, the app shows an 8-digit code.
4. Read all 8 digits to the member who is checking.
5. Select **It matches** only when the member tells you that their code is the same. If it is not the same, select **No match**.

When the member admits you, you join the group.

Compare the code in person, by phone, or on a voice call that BVC does not carry, for example Discord. Do not type the code in game chat.

A request ends after 2 minutes. When it ends, the app shows one of these titles. Select **Ask again** to send a new request.

| Title | Cause |
|---|---|
| **{member} did not admit you** | The member selected **Deny**. |
| **The codes did not match** | You or the member saw a different code. |
| **No member answered** | No member checked your request in time. |
| **No member is online** | No member of the group is online. |

### Ask to join from the game

You can ask to join a Private Group from the game. The code shows only in the BVC app, never in game.

- On Bedrock, type `/bvc:panel`, open **Groups** and select the `[Ask]` row. You can also type `/bvc:groupjoin <code>`.
- On Java, type `/bvc group join <code>`.

The game replies `Your code for <name> is in the BVC app.` Continue in the app from step 2 above. If the BVC app is not open and signed in, the game replies `Open the BVC app to join <name>.`

Asking to join from the game, from the Stream Deck, or from the [WebSocket API](/wiki/creator/websocket-api/) removes you from your current group first, without a confirmation.

### Admit a player

When a player asks to join your Private Group, the app plays a short tone and shows a card: **Admit {player} to {group}?** On desktop, the cards are at the top right of the dashboard, under **Join requests**. On a phone, they open as a sheet. The tone follows the [Mute cues](/wiki/player/settings/#mute-cues) setting.

1. Select **Check code**. The app shows an 8-digit code.
2. Ask the player to read their code to you.
3. Select **Admit {player}** only if it is that player and the code matches. If not, select **Deny**.

The player then confirms on their side, and joins the group.

A check ends after 60 seconds. Select **Not now** to hide a request. It stays until it ends.

Any member who is online can check a request. When another member checks it, your card shows **{member} is checking {player}**.

You can check up to 3 players at the same time. Only one code shows at a time. Select **Show code** on a card to see its code.

If a card shows **Check failed**, do not admit that player. The check could not confirm who sent the request.

:::caution
A name on a card comes from the server. Admit a player only if you know who is reading the code to you.
:::

### Safety code

All members of a Private Group see the same safety code. The code changes when a member joins or leaves.

While you are in a Private Group, the top of the group list shows "Players nearby cannot hear you." To see the code, select **Safety code** on that line. Read it out to the other members. If your code is different, leave the group.

### Leave a Private Group

When you leave a Private Group, the app asks you to confirm:

- **Leave {group}?** To come back, a member must admit you. This applies to the owner too.
- **Close {group}?** You are the last member. If you leave, the group closes.

The app asks when you select the group, select **Leave**, move to another group, or ask to join another Private Group.
