# bedrockvoicechat.com

Marketing site and wiki. Astro, static output, no runtime.

**This branch (`gh-pages-src`) is the source. `gh-pages` is the deliverable.**
Both are orphan branches with no shared history and none with the code branches.
Pushing here builds and publishes to the root of `gh-pages`; see
`.github/workflows/deploy.yml`.

```bash
npm install
npm run dev        # binds 0.0.0.0 — reachable from a phone on the same network
npm run dev:local  # localhost only
npm run build      # -> dist/
npm run preview    # serve the built output, also on 0.0.0.0
npm run check      # typecheck .astro and .ts
```

`npm run dev` prints both a localhost and a network URL. Open the network one on
a phone to check the real thing: the mobile CTA stacking, the rotating word, and
the tap targets are all easier to judge on a device than in a narrow window.

Windows Firewall will usually prompt the first time. If the phone cannot reach
it, allow Node through on the private network, and confirm the laptop and the
phone are on the same subnet (a "guest" SSID is usually isolated).

## Where things are

```
src/
  styles/
    tokens.css        the design system — colour, type, spacing
    base.css          reset, type primitives, audience filtering, reveal
    starlight.css     maps the wiki's variables onto tokens.css
  lib/
    mark.ts           the logo as data (23x13 grid)
    site.ts           versions, URLs, audience list, placeholders
    audience.ts       the who-are-you switcher
    canvas/           one file per visual, each self-registering
  components/         one .astro file per element, styles colocated
  layouts/Base.astro  document shell
  pages/index.astro   THE COMPOSITION — what appears and in what order
  content/docs/wiki/  the wiki, as Markdown
public/               copied verbatim to the site root
```

## Rearranging the home page

`src/pages/index.astro` is the only file that decides what is on the page. Every
block is a component that owns its own styles, so moving, removing or
duplicating one there needs no CSS changes.

Components built and available but not currently placed:

| Component               | What it is                                  |
| ----------------------- | ------------------------------------------- |
| `<Canvas kind="scene">` | Ambient top-down proximity map              |
| `<Canvas kind="band">`  | Full-bleed spectrum rule between sections   |

## Colour

Every accent is a column of the logo. `tokens.css` is the only file allowed to
contain a hex value; everything else refers to a token. The spectrum tokens
(`--sp-violet` through `--sp-crimson`) are sampled from `src/lib/mark.ts`, which
was extracted verbatim from `public/assets/logo/app-logo-transparent.svg`.

If you need a colour that is not in `tokens.css`, it is not in the logo.

## The audience switcher

The page has three tracks — player, operator, creator. All three ship in the
HTML; a `data-audience` attribute on `<html>` decides which is visible, and CSS
does the swapping.

```astro
<Audience for="operator">…</Audience>
<Audience for={['operator', 'creator']}>…</Audience>
```

- No fetch, no loading state, no layout shift. The wrapper is `display:contents`
  when visible, so it never becomes a grid or flex item.
- Works with JavaScript off — the served HTML already has
  `data-audience="player"`.
- Every variant is crawlable.
- `?you=operator` deep-links a track, which is what you would point an ad at.
  The choice persists in `localStorage` after that.

Precedence is URL, then stored choice, then `player`.

## The wiki

Markdown in `src/content/docs/wiki/`. The folder path is the URL — Starlight
0.41 removed `routeBasePath`, and instead derives each route from the file's
location, so nesting under `wiki/` is what produces `/wiki/**`. The marketing
page keeps `/` because a static route in `src/pages` outranks Starlight's
injected `[...slug]`.

Adding a page: create the `.md` file, then add it to `sidebar` in
`astro.config.mjs`. Nothing else. Search (Pagefind) indexes it automatically.

The wiki declares no colours of its own — `starlight.css` only maps Starlight's
variables onto the same tokens the home page uses, so the two cannot drift.

## Deploying

`.github/workflows/deploy.yml` builds this branch and publishes `dist/` to the root
of `gh-pages` on every push.

**gh-pages is a shared branch.** These paths belong to other workflows and must
survive every publish from here:

| Path | Written by | Breaks if deleted |
|---|---|---|
| `api/**` | build-all, release | OpenAPI reference 404s |
| `websocket/**` | build-all, release | AsyncAPI reference 404s |
| `updater/*.json` | release | **auto-update stops for every installed client** |
| `builds.json` | every build | build history |
| `discord/**` | — | OAuth callback |
| `.well-known/**` | — | **iOS/Android deep links stop opening the app** |

Every publisher on this branch uses `keep_files: true`, including ours, because a
root publish with `keep_files: false` deletes all of the above. The two bolded
rows fail silently — nothing errors at deploy time.

The deploy workflow's `KNOWN` list is a tripwire: if a path appears on gh-pages
that neither it nor this build owns, the deploy fails rather than guessing. Add
new CI-owned paths there.

Because `keep_files: true` never deletes, removing the old hand-authored site
(`index.js`, `css/`, `tailwind.config.js`, …) was a one-time manual commit to
`gh-pages`.

## Versions

`src/lib/site.ts` reads `PUBLIC_BVC_VERSION` at build time and falls back to a
hardcoded value.

`release.yml` currently has an `update-gh-pages-version` job that rewrites
version strings on gh-pages directly. That job edits published output, which no
longer has a source of truth behind it once the site is built — it should set
`PUBLIC_BVC_VERSION` and trigger this workflow instead.

## What was dropped from the old site

- **Tailwind** — the `tw-` prefixed utility classes came with the purchased
  template. Component-scoped CSS plus tokens replaced it.
- **GSAP + ScrollTrigger** (two CDN scripts, ~70 KB) — replaced by
  `lib/reveal.ts`, an IntersectionObserver.
- **Bootstrap Icons** (a third CDN stylesheet) — no longer used.

The site now loads no third-party resources at all.

## Still needed before launch

Both are marked with the `.tbd` style wherever they appear, and both live in
`src/lib/site.ts`:

- `DEMO_SERVER.address` — every audience track leads with "hear it before you
  install anything", which is the conversion path the old site did not have.
- `COMMUNITY_SERVER.tier` / `.price` — the Patreon tier granting a seat on the
  community server.
