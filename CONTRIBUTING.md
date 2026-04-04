# Contributing to Bedrock Voice Chat

We welcome contributions! Before submitting your first pull request, please review our [Contributor License Agreement (CLA)](CLA.md).

## Quick Start

1. Read the [CLA](CLA.md) — you only need to do this once
2. Open a GitHub Issue or Discussion describing your proposed change and wait for maintainer approval
3. Fork the repository
4. Create a feature branch
5. Make your changes
6. Sign your commits: `git commit -s -m "Your message"`
7. Submit a pull request linked to the approved issue or discussion

## Discuss Before You Code

Every pull request must be linked to an existing GitHub Issue or Discussion that was approved by a maintainer before work began. This applies to all contributions — features, bug fixes, refactors, and documentation changes.

Pull requests submitted without a linked and approved issue or discussion will be closed without review.

Why? This keeps the project focused, prevents duplicated effort, and ensures changes align with the project's direction. It also gives maintainers a chance to offer guidance before you invest time.

## AI Tooling and Agentic Contribution Policy

We recognize that AI-assisted development tools are part of many developers' workflows. We have no problem with contributors using agentic tools thoughtfully. However, this project is built by and for humans, and we hold all contributions to that standard.

If your contribution was substantially produced by an AI tool, you are still fully responsible for every line of code, every test, and every word in the PR. "The AI generated it" is not a defense for low-quality or broken contributions.

### Requirements for AI-Assisted Contributions

1. Human-in-the-loop is mandatory. A human must review, understand, and validate every change before submission. Autonomous end-to-end generation of PRs is not acceptable.

2. Disclose AI tooling usage. If agentic coding tools (Copilot, Claude Code, Cursor, Aider, etc.) were used to produce a substantial portion of the change, state so in the PR description. This is about transparency, not gatekeeping.

3. You must be able to explain your changes. Maintainers reserve the right to ask you to explain any part of your contribution — the reasoning, the tradeoffs, the implementation details. If you cannot explain what your code does and why, the PR will be closed.

4. Features must be finite in scope and size. Large, sweeping, or unfocused PRs will be closed. Keep changes small, targeted, and reviewable. If a feature is large, break it into incremental PRs discussed in the linked issue.

5. Test coverage is mandatory. All new functionality and bug fixes must include tests. "It works on my machine" is not a test strategy.

6. Human testing of functionality is mandatory. You must have personally tested your changes in a running instance of the application. For minor changes, full UI automation test coverage may substitute, but the expectation is that a human verified the behavior. Describe what you tested in the PR description.

7. Documentation updates are mandatory. If your change affects user-facing behavior, CLI flags, configuration, APIs, or setup steps, update the relevant documentation in the same PR.

8. Security is your responsibility. You are responsible for auditing any AI-suggested code for security vulnerabilities, leaked secrets, hardcoded credentials, and license-incompatible code snippets. PRs that introduce security issues due to unreviewed AI output will be closed and may result in restricted access.

9. Commit messages and PR descriptions must be human-written and meaningful. Generic, obviously-templated, or LLM-boilerplate descriptions (e.g., "This PR improves the codebase by enhancing...") will result in the PR being closed. Describe what you changed, why, and what you tested — in your own words.

### What Is Not Welcome

- LLM-generated feedback, code reviews, or issue comments. Do not paste LLM output as review comments on other contributors' PRs or in issue threads. If we see comments that are clearly LLM-generated boilerplate (e.g., unsolicited refactoring suggestions, generic praise, hallucinated bug reports), they will be deleted and repeat offenders will be blocked.

- Speculative or AI-generated bug reports. Issues must describe behavior you personally observed while using the software. "I asked an LLM to analyze this codebase and it found..." is not a valid bug report. Include concrete reproduction steps from your actual usage.

- Bulk or spray contributions. Submitting multiple low-effort PRs in a short timeframe — especially auto-generated refactors, style changes, or typo fixes across many files — will result in all of them being closed and contributor access being restricted.

- Auto-generated refactoring PRs. Do not point an agentic tool at this repo and submit the output as a PR. Refactoring must be discussed and approved in an issue first, and must demonstrate a clear understanding of the codebase.

### Enforcement

Maintainers will use their judgment. We are not trying to police tools — we are protecting the project and our time. Contributions that do not adhere to these requirements may be enforced by:

1. The PR, issue, or comment being closed or deleted without further discussion
2. Repeat offenders being blocked from the repository

## Signing Your Commits

All commits must include a `Signed-off-by` line certifying that you agree to the [CLA](CLA.md):

```bash
git commit -s -m "Add feature X"
```

This adds:

```
Signed-off-by: Your Name <your.email@example.com>
```

This certifies that you have the right to contribute the code and agree to our CLA terms.

## Corporate Contributions

If you're contributing on behalf of a company, please contact us about signing a Corporate CLA before submitting code.

## License

The license terms of this project are outlined in LICENSE.md