# Slides

A recordable slide deck in the site's own design language. `deck.html` links
`../src/styles/tokens.css`, so the deck cannot drift from bedrockvoicechat.com.

## Present

Open `deck.html` in any browser.

| Key | Does |
|---|---|
| `→` `space` `PageDown` click | Next slide |
| `←` `PageUp` `Backspace` | Previous slide |
| `Home` `End` | First, last |
| `F` | Fullscreen |
| `H` | Hide the footer and progress bar |

`?s=7` opens on slide 7. `?chrome=0` opens with the footer already hidden.

The stage is a fixed 1920x1080 box scaled by one transform, so the frame is
pixel-identical at any window size. A window that is not 16:9 pillarboxes.

## Export

```bash
npm run slides:export              # 3840 x 2160 into slides/out
npm run slides:export -- --scale=1 # 1920 x 1080
```

| Flag | Does |
|---|---|
| `--scale=N` | 1 to 4. Multiplies 1920x1080. Default 2 |
| `--out=DIR` | Write somewhere other than `slides/out` |
| `--only=6,7` | Export only those slide numbers |
| `--clean` | Delete the previous export first |
| `--chrome` | Keep the footer and progress bar. Off by default |

The export drives whatever Chromium is installed — Edge on Windows, Chrome
elsewhere. Set `BVC_SLIDES_BROWSER` to override the executable. Nothing is
downloaded and no package is added.

Filenames come from each slide's `data-name`, so a reordered deck still exports
under names that mean something.

## Edit

One `<section class="slide" data-layout="..." data-name="...">` per slide.

| Layout | For |
|---|---|
| `title` | Opening and closing frames. Carries the mark |
| `break` | A part divider: eyebrow, statement, spectrum rule |
| `text` | A headline and a list of rows. Add `tight` to `.rows` for four or more |
| `code` | A headline and one code panel |
| `split` | Copy left, code panel right |

Code colour is marked by hand: `cm` comment, `cd` command, `fl` flag, `ky` key,
`st` string, `nu` number, `pn` bracket. Each maps to one stop of the logo
spectrum. Add `sm` or `xs` to a `.panel` when a block would overrun the frame.

`.note` is the yellow line a viewer must act on. `.note.quiet` is an aside.
