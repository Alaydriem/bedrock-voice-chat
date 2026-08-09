---
title: Reporting an issue
description: Where to report a bug or ask a question, and what to include.
sidebar:
  label: Reporting an issue
  order: 3
---

**[Discord](https://discord.gg/CdtchD5zxr) is where support happens.** Post there. It gets triaged into a GitHub issue if it is a bug, or answered on the spot if it is not.

You do not need to work out which it is first. That is the triage step.

:::tip[Check the troubleshooting page first]
Most reports resolve there in under a minute.

[Players](/wiki/player/troubleshooting/) · [Operators](/wiki/server/troubleshooting/)
:::

## What to include

**If you are a player:**

- Your platform. Windows, macOS, Linux, Android, iOS, or console plus which phone
- The error code, if the app showed one
- The server you are on, and whether anyone else there has it working
- What you already tried

**If you run a server:**

- Output of `curl https://<your-server>/api/config`
- Whether **UDP** is open, not just TCP. This is the most common cause
- Whether `curl -H "X-MC-Access-Token: <token>" https://<your-server>/api/position` returns data
- How you are deployed. Docker, bare metal, or embedded in the Java mod

**For a crash:** what you were doing, whether it reproduces, and the log if you have it.

## Where things go

| | Where |
|---|---|
| Anything at all | [Discord](https://discord.gg/CdtchD5zxr) |
| Confirmed bugs | [GitHub Issues](https://github.com/Alaydriem/bedrock-voice-chat/issues), usually opened during triage |
| Feature requests | Discord or GitHub Issues |
| Longer technical discussion | [GitHub Discussions](https://github.com/Alaydriem/bedrock-voice-chat/discussions) |

Search before posting.

## Security issues

:::danger[Do not open a public issue for a security problem]
Use GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) under the Security tab.
:::

There is no bug bounty. Legitimate reports are credited in `SECURITY.md`.

## Helping others

Answer a question on Discord when you know the answer. See [supporting BVC](/wiki/start/supporting-bvc/).
