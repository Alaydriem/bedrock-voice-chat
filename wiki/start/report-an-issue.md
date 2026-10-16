---
title: "Reporting an issue"
description: "Where to report a bug or ask a question, and what to include."
source: https://www.bedrockvoicechat.com/wiki/start/report-an-issue/
---
For help, join our **[Discord](https://discord.gg/CdtchD5zxr)**.

:::tip[Check the troubleshooting page first]
The troubleshooting pages have fixes for the most common problems.

[Players](/wiki/player/troubleshooting/) · [Operators](/wiki/server/troubleshooting/)
:::

## What to include

**If you are a player:**

- Your platform: Windows, macOS, Linux, Android or iOS. If you play on a console, give the console and the phone or tablet that runs the app.
- The error code, if the app showed one.
- The server that you play on, and if other players on that server can use voice chat.
- The fixes that you already tried.

**If you run a server:**

- The output of `curl https://<your-server>/api/config`.
- If the UDP voice port is open, as well as TCP/443. A closed UDP port is the most common cause of missing audio.
- If `curl -H "Authorization: Bearer <token>" https://<your-server>/api/position` returns data. `<token>` is the value of `server.minecraft.access_token`.
- How you run the BVC server: Docker, directly on the host, or inside the Java mod.

**If the app crashed:** tell us what you were doing, and if the crash happens again. Attach the log file if you have it.

## Where to report

| Report | Where |
|---|---|
| Any question or problem | [Discord](https://discord.gg/CdtchD5zxr) |
| Confirmed bugs | [GitHub Issues](https://github.com/Alaydriem/bedrock-voice-chat/issues). The maintainers usually open these from Discord reports. |
| Feature requests | Discord or GitHub Issues |
| Long technical discussion | [GitHub Discussions](https://github.com/Alaydriem/bedrock-voice-chat/discussions) |

Search for your problem before you post.

## Security issues

:::danger[Do not open a public issue for a security problem]
Use GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) under the Security tab.
:::

There is no bug bounty. We credit valid reports in `SECURITY.md`.

## Helping others

If you know the answer to a question on Discord, answer it. See [supporting BVC](/wiki/start/supporting-bvc/).