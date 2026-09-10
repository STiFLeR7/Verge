# Tool identity marks — provenance

Real, verifiable brand marks for design-reference purposes only (per this
session's explicit scope decision: `docs/design/*.html` mockups, **not**
bundled into the Verge application). Do not copy these into `platform/`,
`ui/`, or any shipped asset path without a separate, deliberate licensing
review — see "Before any production use" below.

## Source

Fetched 2026-09-08 from the `lobehub/lobe-icons` project
(`github.com/lobehub/lobe-icons`, MIT-licensed icon set), via its
`@lobehub/icons-static-svg` npm package, served through the jsDelivr CDN:

```
https://cdn.jsdelivr.net/npm/@lobehub/icons-static-svg@latest/icons/<slug>.svg
```

Each file below was fetched directly (not hand-recreated) and verified by
its embedded `<title>` element matching the intended tool:

| File | Slug used | `<title>` in the file | Tool |
|---|---|---|---|
| `claudecode-color.svg` | `claudecode-color` | Claude Code | Anthropic's Claude Code |
| `codex.svg` | `codex` | Codex | OpenAI Codex |
| `grok.svg` | `grok` | Grok | xAI Grok |
| `pi.svg` | `pi` | Pi | Inflection AI's Pi |
| `opencode.svg` | `opencode` | opencode | sst/opencode |
| `hermesagent.svg` | `hermesagent` | Hermes Agent | Nous Research's Hermes Agent |
| `antigravity-color.svg` | `antigravity-color` | Antigravity | Google Antigravity |
| `kilocode.svg` | `kilocode` | Kilo Code | Kilo-Org/kilocode |

`claudecode-color.svg`'s fill (`#D97757`) independently confirms the brand
color already used (as an unverified approximation, `(218,119,86)` /
`#DA7756`) in `ui/ambient/windows/src/lib.rs`'s `identity()` function —
the two are within one unit per channel, close enough that no code change
is needed there, but this is now a verified fact rather than a guess.

## What these are, honestly

- **Trademarks, not freely-licensed brand assets.** The MIT license on
  `lobe-icons` covers the SVG *markup* lobehub authored/collected; it does
  not grant a trademark license to Anthropic's, OpenAI's, xAI's, Google's,
  or anyone else's actual brand identity. Each company's own brand
  guidelines still govern acceptable use — most restrict implying
  endorsement, sponsorship, or an official partnership.
- **Monochrome by design for most tools.** `codex.svg`, `grok.svg`,
  `pi.svg`, `opencode.svg`, `hermesagent.svg`, and `kilocode.svg` all use
  `fill="currentColor"` — they are genuinely single-color marks in their
  real branding, not a limitation of this fetch. Only Claude Code
  (`#D97757`) and Antigravity (a multi-color gradient composition) have
  official color treatments.
- **`antigravity-color.svg` contains its own `<defs>`/filter IDs**
  (`lobe-icons-antigravity-*`). If it is ever inlined (not referenced via
  `<img>`) more than once on the same page, those IDs will collide — keep
  it to one inlined instance per document, or reference it via `<img src>`
  instead.

## Before any production use

Do not treat "found via lobe-icons" as clearance to ship these inside the
Verge application. A real production decision needs, at minimum: checking
each company's current brand guidelines for third-party/compatibility-
badge usage, deciding whether Verge's use case (identifying which tool an
ambient indicator refers to) qualifies as permitted nominative use in each
company's specific policy, and likely reaching out for explicit permission
where guidelines are ambiguous or restrictive. This is a real, unresolved
open item, not a formality — flagged here rather than decided by default.
