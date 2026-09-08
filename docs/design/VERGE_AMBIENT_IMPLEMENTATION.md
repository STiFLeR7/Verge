# Verge Windows Ambient Surface — Implementation Notes

Maps `docs/design/VERGE_AMBIENT_DESIGN.md` (the visual source of truth) to
the actual Windows implementation in `platform/windows/src/overlay.rs` and
`ui/ambient/windows/src/lib.rs`. Windows only, per this task's scope — no
Linux/macOS work, no Tauri, no new tools.

## What was implemented

- A real, right-edge-flush, vertical capsule, rendered with genuine
  per-pixel alpha (`UpdateLayeredWindow` + a hand-written software
  rasterizer), replacing the first vertical slice's color-key transparency.
- Rounded left corners only, sharp/flush right edge (design §3).
- A horizontal translucency gradient (denser near the screen edge, lighter
  toward the inner edge) and a 1px hairline highlight on the outer edge
  (design §14).
- A neutral, un-filled identity ring around a tool's badge when a real
  metric exists (design §9) — never a percentage arc for a bare count.
- Hover-driven compact ↔ expanded transition, growing the same window
  leftward (design §17), not a separate popup.
- An idle sliver (5×64px, low alpha) when no account is discoverable at
  all (design §4).
- Honest, jargon-free copy for every real `Availability` outcome the
  Claude Code adapter can actually produce.
- A density cap (4 glyphs) and `+N` overflow mechanism in
  `ui/ambient/windows::render`, architecturally present even though
  exactly one real tool exists today.

## Mapping: design spec → code

| Design spec section | Implementation |
|---|---|
| §3 Right-edge geometry | `platform/windows/src/overlay.rs`: window is resized/repositioned via `UpdateLayeredWindow` every render so its right edge always equals `GetSystemMetrics(SM_CXSCREEN)`; `CORNER_RADIUS` applied only to the two left corners in `rounded_rect_coverage` |
| §4 Idle state | `paint_idle` — a 5×64px sliver at `IDLE_ALPHA = 60/255`, drawn whenever `OverlayContent::glyphs` is empty |
| §5 Active state | `paint_capsule`'s badge + optional `has_metric` ring; no text in the compact view |
| §6 Multi-agent / overflow | `ui/ambient/windows::render`'s `GLYPH_CAP = 4` and `overflow_count`; drawn as a small "+N" mark in `paint_capsule`'s overflow branch |
| §7/§8 Permission / completed tint | `StateTint` enum and `tint_rgb()` exist and are drawn (a thin colored ring outside the badge) — **always `Neutral` today**, see "Activity Gap" below |
| §9 Usage, visual-first | `has_metric` gates the neutral ring; a `Fraction` reading would additionally get a filled percentage arc — not yet exercised by real data (Claude Code's real signal is a bare `Count`) |
| §10 One limit line only | `ui/ambient/windows::render_one` emits at most two `detail_lines` for a `Count` reading (the count itself, and "No published limit") — never a per-model list |
| §13 Color hierarchy | `MATERIAL_RGB` is neutral near-black; `brand_color` comes from `identity()`, never recolored; `tint_rgb` is applied only as a thin secondary ring, never a fill |
| §14 Liquid Glass | `paint_capsule`'s gradient + hairline highlight + soft anti-aliased rounded-rect coverage (`rounded_rect_coverage`) — see "Material technique" below for what was and wasn't attempted |
| §15 Typography | `make_font` uses Segoe UI at 11/14/18-equivalent sizes (`size_px` arguments: 11 for meta lines, 14 for the tool name, 20 for the badge mark) |
| §16 Motion | Deliberately minimal this pass — see "Motion" below |
| §17 Expansion | Hover poll (`TIMER_HOVER`, 50ms) toggles `WindowState::expanded`, which changes `compute_layout`'s `text_column_width` from 0 to a measured value; the glyph column never moves, only the text column grows/shrinks to its left |
| §18 Density | `GLYPH_CAP`, `GLYPH_SLOT_HEIGHT`, `GLYPH_GAP_V` |
| §19 Multi-DPI | `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` preserved from the first slice; multi-monitor placement genuinely not implemented — see "Known limitations" |
| §21 No implementation language | `ui/ambient/windows::render_one` strips the domain's own technical qualifier out of `UsageWindow::name`, and never echoes `Availability::Unsupported`'s raw `reason` or `Error`'s raw `diagnostic` |

## Real data currently available

Exactly one real signal, unchanged from the first vertical slice: Claude
Code's local `stats-cache.json` bare message count, gated by a real
(if minimal) check against `.credentials.json` via `WindowsCredentialStore`.
Confirmed live during this task's own verification: the expanded capsule
rendered `Claude / 2456 messages on 2026-08-20 / No published limit` —
real numbers read from this machine's actual installation, not fabricated
for a screenshot (see `docs/design/evidence/ambient_capsule_expanded.png`).

## States currently supported (backed by real data)

- Idle sliver (no account discoverable) — verified via a disposable,
  uncommitted example harness (`OverlayContent::default()`), since a real
  account is always discoverable on this development machine; deleted
  before this task's commit.
- Compact badge with a neutral metric ring (real `Available` + `Count` case).
- Expanded detail panel with real count, real (approximate) date, and the
  honest "No published limit" line.
- `Unauthenticated` / `AccessDenied` / `NotMetered` / `Unsupported` /
  `Unreachable` / `Error` copy paths exist and are unit-tested
  (`ui/ambient/windows`'s test suite), but were not all individually
  screenshotted against a live, artificially-forced condition this pass —
  their correctness rests on the unit tests plus direct code review, not
  a live visual capture for every branch.

## States architecturally prepared but not yet backed by real data

- **`StateTint::Working` / `Waiting` / `Completed`** — the enum, the
  drawing code (a secondary tinted ring), and the semantic mapping all
  exist. Every real glyph renders `Neutral` today because no
  `ActivitySource` exists for any tool (see "Activity Gap" below). This is
  not a partial implementation of activity — it is a complete rendering
  path with no real input wired to it yet, by design, not by oversight.
- **Session count** (design §11) — no `ActivitySession` data exists, so no
  session count is rendered anywhere, compact or expanded. Nothing in the
  code fabricates one.
- **Percentage-fill usage ring** — the drawing code only ever receives
  `has_metric = false` today (Claude Code's real signal has no fraction to
  show); a future tool or a future Claude official-endpoint `UsageSource`
  that does return a `Fraction` would light up the existing fill-arc path
  without any rendering code changing.
- **Multiple simultaneous tools / `+N` overflow** — `render()` takes a
  slice and caps it correctly (unit-tested with 6 synthetic states in
  `overflow_caps_the_visible_glyph_count`), but `apps/desktop/src/main.rs`
  only ever constructs one real `AmbientState` today, so this never
  triggers in the running application.

## Activity Gap (explicit, per this task's own instruction)

`core::domain::ActivityState` has exactly three variants —`Working`,
`WaitingOnUser`, `RecentlyIdle` — and no distinct "completed/terminated"
value, and no `ActivitySource` implementation exists for any tool. The
domain was **not modified** to make the green/yellow/red design possible;
this task's brief was explicit that this must not happen. Consequently:

- Green (working), yellow (waiting for permission), and red
  (completed/stopped) are all real, drawn, tested code paths — none of
  them fire in the current running application, because none has a real
  data source. `StateTint::Neutral` is what every real glyph gets.
- This is documented rather than simulated: no fake "Working" state is
  ever constructed to make a screenshot look more finished than the real
  system is.

## Material technique (what "Liquid Glass" actually means here)

Implemented: real per-pixel alpha (not color-key), soft anti-aliased
rounded corners via a signed-distance-function rasterizer, a directional
translucency gradient, a hairline edge highlight, and anti-aliased text/
glyph rendering via the standard "render white-on-black, use luminance as
alpha" GDI technique (there is no native way to get anti-aliased GDI text
directly onto an alpha-aware surface, so this is the well-established
workaround, not an invented hack).

**Not implemented, and explicitly not claimed:** real backdrop blur
(sampling and blurring the actual desktop content behind the capsule).
Windows' documented mechanism for that (`DWM` blur-behind /
`SetWindowCompositionAttribute`-style APIs) is partially undocumented and
version-fragile, and layering it under an already-custom alpha-blended
`UpdateLayeredWindow` surface introduces real compositor-interaction risk
that has not been evaluated. The current material achieves depth and
translucency through gradient + alpha + a highlight, not through sampling
what's actually behind it. This is a real, named gap against the "material
reacting to the environment" language in the design spec, not a silent
substitution.

**Not implemented:** true vector/bitmap brand logos. `identity()` in
`ui/ambient/windows/src/lib.rs` uses a single Unicode character per tool
(`✳` for Claude) rendered via the same text-alpha technique as body copy,
with an approximated, unverified brand color. This is a stand-in, clearly
labeled as such in code comments, not a claim that these are the tools'
real marks.

## Motion

Implemented: the compact ↔ expanded transition (an immediate resize +
redraw on the hover poll's 50ms cadence — no eased/animated interpolation
between the two sizes yet, so the transition is a discrete jump, not the
smooth morph design §16 describes). This is a real, honest simplification:
animating a live `UpdateLayeredWindow`-based resize smoothly would need an
interpolation loop driving intermediate `present()` calls, which was not
built this pass. Everything else in §16 (crossfading tints, a one-time
permission attention cue, breathing on the working state) has nothing to
animate yet, since no `ActivityState` signal exists (see "Activity Gap").

## Known limitations (explicit, not silently assumed away)

- **Multi-monitor placement is unverified.** The capsule always positions
  against `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)` (the primary
  display's dimensions). On a multi-monitor system this places it on the
  primary display only; behavior when the primary display isn't the
  "active" one, or when a display is added/removed while running, was not
  tested — consistent with `docs/STATUS.md`'s existing, standing
  multi-monitor gap, not a new one introduced here.
  - This was tested at the one hardware configuration available for this
    task: a single 1920×1200 physical display at 125% (120 DPI) scaling.
    Per-Monitor-V2 DPI awareness is declared, but a second monitor at a
    *different* scale factor was not available to verify against.
- **No real click target.** The whole surface is permanently click-through
  (`WS_EX_TRANSPARENT` held on, never toggled) — see the module doc
  comment in `overlay.rs` for why this is a deliberate scope decision, not
  a regression: nothing in this pass needs to intercept a click, since
  expand/collapse is hover-driven. Do not casually add a click handler
  without re-introducing the validated dynamic-toggle mechanism the first
  slice proved necessary for genuine cross-process click delivery.
- **A real, non-obvious bug was found and fixed during this task's own
  bring-up**, worth flagging for anyone extending this file: constructing
  `WindowState<F>` with the real (unboxed) closure type, then reading it
  back through a raw pointer cast to `WindowState<Box<dyn Fn() -> OverlayContent>>`,
  is undefined behavior — the two types have different memory layouts. The
  closure **must** be boxed into `Box<dyn Fn() -> OverlayContent>` before
  constructing `WindowState`, exactly as the first slice's original code
  did and this rewrite briefly, incorrectly, stopped doing. It manifested
  as silently-wrong `Cell<bool>` reads, not a clean panic — a genuinely
  dangerous failure mode to keep in mind for any future refactor of this
  file's state-passing.
- **No native accessibility exposure** — flagged in the design spec (§20)
  as future work, still true; not attempted this pass.
- **Not every `Availability` branch was individually screenshotted** —
  `Unauthenticated`/`AccessDenied`/etc. render via unit-tested formatting
  logic (`ui/ambient/windows`'s 8 tests) but weren't each forced into the
  live running app for a visual capture, since doing so would require
  either fabricating credential state or temporarily breaking the real
  installation on this machine — neither was judged worth the risk for
  this pass.

## What must not be changed casually

- The closure-boxing requirement above (`Box<dyn Fn() -> OverlayContent>`
  before constructing `WindowState`) — removing it reintroduces silent
  memory corruption, not a compile error.
- `SWP_NOMOVE | SWP_NOSIZE` on the periodic topmost-reassertion
  `SetWindowPos` call — omitting them collapses the window to a zero rect
  (the first slice's own documented finding, still true here).
- The permanent `WS_EX_TRANSPARENT` — do not add a click handler without
  first re-reading the first slice's `WM_NCHITTEST`/`HTTRANSPARENT`
  failure and re-implementing the validated cursor-poll-driven dynamic
  toggle, not a naive hit-test.
- The `identity()` function's exhaustive match over `ToolId` — every
  variant needs a real entry so the render pipeline never needs an
  `if`/`match` fallback branch when a second real tool is wired up on
  Windows.
