---
title: Reporting an issue
description: Where to report a bug or ask a question, and what to include.
sidebar:
  label: Reporting an issue
  order: 3
---

**[Discord](https://discord.gg/CdtchD5zxr) is where support happens.** Post there and it gets triaged from there — into a GitHub issue if it turns out to be a bug, or answered on the spot if it does not.

You do not need to work out which it is first. That is the triage step, and it is not your job.

:::tip[Check the troubleshooting page first]
Not gatekeeping — most reports resolve there in under a minute, and the ones that do not are much easier to help with once you have ruled a few things out.

[Players](/wiki/player/troubleshooting/) · [Operators](/wiki/server/troubleshooting/)
:::

## What to include

The difference between a fast answer and a long back-and-forth is usually three facts.

**If you are a player:**

- Your platform — Windows, macOS, Linux, Android, iOS, or console plus which phone
- The server you are on, and whether anyone else there has it working
- What you already tried

**If you run a server:**

- Output of `curl https://<your-server>/api/config`
- Whether **UDP** is open, not just TCP — this is the most common cause and the easiest to overlook
- Whether `curl -H "X-MC-Access-Token: <token>" https://<your-server>/api/position` returns data
- How you are deployed — Docker, bare metal, or embedded in the Java mod

**For a crash:** what you were doing, whether it reproduces, and the log if you have it.

## Where things go

| | Where |
|---|---|
| Anything at all | [Discord](https://discord.gg/CdtchD5zxr) |
| Confirmed bugs | [GitHub Issues](https://github.com/Alaydriem/bedrock-voice-chat/issues) — usually opened during triage |
| Feature requests | Discord or GitHub Issues |
| Longer technical discussion | [GitHub Discussions](https://github.com/Alaydriem/bedrock-voice-chat/discussions) |

Search before posting. The same handful of questions come up often, and the answer is usually already there.

## Security issues

:::danger[Do not open a public issue for a security problem]
Use GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) under the Security tab.
:::

There is no bug bounty. Legitimate reports are credited in `SECURITY.md`.

## Helping others

Answering a question on Discord when you know the answer is the most useful thing anyone does here. See [supporting BVC](/wiki/start/supporting-bvc/).
