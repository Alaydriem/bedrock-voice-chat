---
title: Parents and age settings
description: Age Signals on Android and Declared Age Ranges on iOS — what they do and how to change them.
sidebar:
  label: Parents and age
  order: 9
---

BVC implements Age Signals on Android and Declared Age Ranges on iOS so parents can decide whether their child uses voice chat on the servers they play on.

## What BVC is

A proximity voice chat app for Minecraft Bedrock and Java. It lets players talk to each other across platforms.

:::caution[Who runs the server your child connects to]
Bedrock Voice Chat is software, not a service. Each server is set up and run by the
Minecraft community that owns it — a friend, a school group, a Discord community, whoever
runs the Minecraft server your child plays on.

Their operator controls the whitelist, the minimum age, and who else is on it. We do not
run those servers, cannot see who is on them, and cannot moderate them. If something goes
wrong on a server, the operator is who to contact.
:::

## What information is collected

**Your child's gamertag and profile picture URL are sent to the server they connect to.**
That is how other players see who is speaking, and how the app shows names and avatars in
the player list. Without it, voice chat would be anonymous audio with no way to tell who
is who — or to mute someone.

That information goes to **the server operator's server**, not to us. What they do with
it is governed by their own practices.

Diagnostic logs, error logs, and some event tracking go to analytics providers, used only
to improve the app. None of that identifies you or your child, and it can be
[turned off by the operator](/wiki/server/privacy-and-telemetry/).

Your child's age is never stored or transmitted. BVC only asks Apple or Google whether
the signed-in user clears the server's minimum, and receives yes or no.

Full policy: [PRIVACY_STATEMENT.md](https://github.com/Alaydriem/bedrock-voice-chat/blob/master/PRIVACY_STATEMENT.md).

## Who your child can talk to

Whoever runs the server. Operators set the minimum age for their own community and BVC
honours it, but who is actually on a server is that community's business — often a group
of friends from school, sometimes a wider online community.

Because the operator is a community member rather than a company, the standards vary. Some
servers are moderated carefully and some are not. Worth knowing who your child is playing
with, and worth agreeing together on what is fine to share online.

## Blocking someone

You can mute individual players in the app. A muted player cannot be heard, even in proximity range.

If someone is behaving badly, contact the server operator. That kind of thing usually breaches the community's own rules and their moderators can act on it.

## When the age warning appears

Two things both have to be true:

1. The server operator has set a minimum age.
2. Your device and Google or Apple account are in a region where Age Signals or Declared Age Ranges are required.

If either is false, no warning appears.

## Controls outside those regions

BVC is whitelist-based. Your child cannot join voice chat on a server unless that server's operator has explicitly added them.

If you would rather they did not use it at all, device-level parental controls and app installation restrictions are the way to enforce that.

### Restricting to specific servers

Not currently possible. If you want it, open a GitHub issue or raise it on Discord.

## Changing the setting

BVC cannot unlock this and has no override. It asks your platform whether the signed-in user is above the server's minimum age. The answer comes from Google or Apple and changes in their parental controls.

Once the platform reports differently, restart BVC and it re-checks.

### Android

Age Signals come from the Google account on the device and any supervision through Family Link.

1. Open the child's profile in **Family Link**.
2. Check the birth date is right. A wrong birth date is the usual cause of an unexpected block.
3. Grant the consent Google prompts for when a supervised account uses an app with communication features.

If the account is unsupervised and the birth date is correct, the block is the server's minimum age, not Google's. Ask the operator what they set.

Google's docs: [Family Link Help](https://support.google.com/families).

### iOS

Declared Age Ranges come from the Apple Account's date of birth and Family Sharing.

1. Open the child's account under **Settings → Family**.
2. Check the date of birth.
3. Review **Screen Time → Content & Privacy Restrictions** and allow access to the declared age range when prompted.

Same as Android: if the account is not in a Family Sharing group and the date of birth is right, it is the server's setting.

Apple's docs: [Family Sharing](https://support.apple.com/en-us/HT201060).

## For operators

Set it in `config.hcl`:

```hcl
server {
    age {
        minimum = 13
    }
}
```

`0` disables the check. Anything else is clamped to 13–18. Unset defaults to 13. See [privacy and telemetry](/wiki/server/privacy-and-telemetry/).
