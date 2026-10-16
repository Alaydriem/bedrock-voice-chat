---
title: Parents and age settings
description: How Age Signals on Android and Declared Age Ranges on iOS work, and how to change them.
sidebar:
  label: Parents and age
  order: 9
---

Bedrock Voice Chat (BVC) uses Age Signals on Android and Declared Age Ranges on iOS. These are age checks from Google and Apple. Parents decide if their child uses voice chat on the servers that the child plays on.

## What BVC is

Bedrock Voice Chat is a voice chat app for Minecraft Bedrock and Java. Players hear other players who are near them in the game. Players on different devices can talk to each other.

:::caution[Who runs the server your child connects to]
Bedrock Voice Chat is software. Other people install it on their own Minecraft servers. We do not run these servers. Each server belongs to a Minecraft community, for example a friend, a school group, or a Discord community.

The person who runs the server (the operator) decides which players can join, and sets the minimum age. We cannot see who is on these servers, and we cannot moderate them. If something goes wrong on a server, contact its operator.
:::

## What information is collected

**The app sends your child's gamertag and profile picture URL to the server that they connect to.** Other players see who is speaking, and the app shows names and pictures in the player list.

This information goes to the operator's server. It does not go to us. The operator decides what to do with it.

Bedrock Voice Chat collects only anonymous analytics: diagnostic logs, error logs, and some usage events. We use this data only to improve the app. None of it identifies you or your child. The operator can [turn it off](/wiki/server/privacy-and-telemetry/).

BVC never stores or sends your child's age. BVC asks Apple or Google if the signed-in user meets the minimum age of the server. Apple or Google answers only yes or no.

See the full policy: [PRIVACY_STATEMENT.md](https://github.com/Alaydriem/bedrock-voice-chat/blob/master/PRIVACY_STATEMENT.md).

## Who your child can talk to

Your child can talk to all players on the server. The operator sets a minimum age for the server, and BVC applies it. The community decides who plays on its server. Often it is a group of friends from school. Sometimes it is a larger online community.

The operator is a member of the community, and each community has different rules. Some servers have careful moderation and some do not. Find out who your child plays with. Agree together on what is safe to share online.

## Block someone

You can mute a player in the app. Your child then does not hear that player, even when the player is near. The muted player can still hear your child.

If a player behaves badly, contact the server operator. Bad behaviour usually breaks the rules of the community, and the community's moderators can act on it.

## When the age warning appears

The age warning appears only when both of these are true:

1. The server operator has set a minimum age.
2. Your device and Google or Apple account are in a region that requires Age Signals or Declared Age Ranges.

If either is false, no warning appears.

## If no age warning appears

Your child cannot use voice chat on a server until the operator of that server adds them, or until your child joins the server in Minecraft.

To stop all use of the app, use the parental controls and app restrictions on the device.

### Limit to specific servers

You cannot limit BVC to specific servers at this time. To ask for this feature, open a GitHub issue or ask on Discord.

## Change the setting

You cannot change this setting in BVC. BVC asks Google or Apple if the signed-in user is old enough for the server. To change the answer, change the parental controls on the Google or Apple account.

After you change the parental controls, restart BVC. The app checks again when it starts.

### Android

Age Signals come from the Google account on the device, and from Family Link supervision.

1. Open the child's profile in **Family Link**.
2. Check the birth date. A wrong birth date is the usual cause of an unexpected block.
3. If Google asks for consent for this app, give it. Google asks for consent when a supervised account uses an app that lets users talk to other people.

If the account is not supervised and the birth date is correct, the cause of the block is the minimum age that the operator set. Ask the operator what age they set.

Google's help: [Family Link Help](https://support.google.com/families).

### iOS

Declared Age Ranges come from the date of birth of the Apple Account, and from Family Sharing.

1. Open the child's account under **Settings → Family**.
2. Check the date of birth.
3. Open **Screen Time → Content & Privacy Restrictions**. When iOS asks, allow access to the declared age range.

If the account is not in a Family Sharing group and the date of birth is correct, the cause of the block is the minimum age that the operator set. Ask the operator what age they set.

Apple's help: [Family Sharing](https://support.apple.com/en-us/HT201060).

## For operators

See [Minimum age](/wiki/server/privacy-and-telemetry/#minimum-age).
