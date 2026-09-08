# Verge Ambient Surface — Design Specification

**Status: design only.** Nothing in this document has been implemented.
Nothing in `core/`, `tools/`, or `platform/` changes as a result of this
file. This is the visual/interaction specification the eventual
`ui/ambient/*` implementations must satisfy, written against the
architecture and domain model that already exist (`docs/PRODUCT_ARCHITECTURE.md`,
`docs/STATUS.md`, `core/src/domain/*`) — not a new architecture, and not a
license to add anything those documents don't already allow.

---

## 1. Design Philosophy

> **Ambient instrument, not dashboard.**

Verge's entire reason to exist is that checking it costs nothing
(`docs/PRODUCT_ARCHITECTURE.md` §1). A dashboard demands attention to
justify its own screen space — text to read, charts to interpret, cards to
scan. An instrument does the opposite: it sits at the edge of perception
and only asks for attention when something has genuinely changed. A car's
oil-pressure light is not a dashboard; it is dark almost all the time, and
that darkness is not a missing feature, it is the entire value proposition.

Concretely, this means:

- **Restraint is the primary design material.** Every pixel Verge draws
  when nothing needs attention is a pixel arguing against the product's own
  premise.
- **The UI is a side effect of state, not a container waiting to be
  filled.** There is no "empty state" to design as an afterthought — the
  quiet, idle presence *is* the normal state, and the busy states are the
  exception being rendered into it.
- **Success is being forgotten.** If a user can describe what Verge looked
  like five minutes ago without having looked at it, the design has failed
  in the specific way this product cannot afford to fail.
- **Confidence over decoration.** The inspiration image (§2) demonstrates
  this directly: it has no border, no shadow, no gradient, and no filler
  text, and it does not need any of those things to look intentional. A
  design that needs a drop shadow to prove it is "there" has not earned its
  place on the desktop yet.

---

## 2. Inspiration Analysis

The provided reference is a compact activity indicator merged with a
device's hardware notch, showing two lines: a dim meta line ("Read
app-sidebar.tsx  219 lines") and a bold primary line ("Thinking") paired
with a small, true-color brand icon. It is explicitly **not** being copied
as a component — the geometry it uses (top-center, notch-merged, horizontal
pill) is exactly what this document's brief rules out for Verge (§3). What
is being extracted is the *design thinking* underneath it:

| Quality observed | What it demonstrates | How Verge inherits it |
|---|---|---|
| Exactly two lines of text, nothing else | Density is a discipline, not a limitation of space | The ambient surface never shows more than one meta value + one state per visible item (§9, §10, §18) |
| One small, true-brand-color icon | Identity does the work color-coding would otherwise do | Verge never invents its own agent iconography (§12, §13) |
| Zero border, zero shadow, zero gradient | The material itself communicates "this belongs here," not decoration around it | Liquid Glass system (§14) explicitly forbids visible glass borders and drop shadows |
| Optically merges with the physical device feature it sits beside | The surface reads as *attached*, not *floating* | Verge's right-edge surface is specified to feel structurally attached to the display edge (§3), not a window with visible margin |
| Dim/bold two-tier typography, no size extremes | Hierarchy comes from weight and value contrast, not scale | Typography system (§15) reuses a restrained two-to-three-tier scale, no oversized headings |
| A single word ("Thinking") stands in for a whole process | Trusts the user to infer meaning from minimal language | Verge's working state needs no label at all in the ambient view — color + icon + subtle motion carry it (§5, §16) |
| No numbers, no percentages, no progress bar visible in the glance state | Confidence that *state* matters more than *precision* at a glance | Usage is visual-first in the ambient view; numbers exist only on deliberate expansion (§9) |

The one thing explicitly **not** inherited: its geometry. A horizontal,
top-center, notch-merged pill is a macOS-specific, hardware-specific trick.
Verge's own identity (per this brief, and per `docs/PRODUCT_ARCHITECTURE.md`
§2's rejection of the hardware-notch-as-identity) is the vertical,
right-edge-mounted surface specified below.

---

## 3. Right-Edge Surface Geometry

The ambient surface is a **vertical capsule mounted flush to the right edge
of the display**, not a floating window with visible margin on any side.
It auto-sizes to its own content along the vertical axis and never occupies
more horizontal space than the content it is currently showing requires.

```
Idle (nothing active):

┌──────────────────────────────────────┐
│                                      ▐│  ← a few px wide, low-contrast
│                                      ▐│    sliver, flush to the edge
│                                      ▐│
│                                      ▐│
└──────────────────────────────────────┘

One tool active:

┌──────────────────────────────────────┐
│                                     ╭─┤
│                                     │●│  ← capsule grows just enough
│                                     ╰─┤    to hold one glyph
│                                      ▐│
└──────────────────────────────────────┘

Three tools, one needs permission:

┌──────────────────────────────────────┐
│                                     ╭─┤
│                                     │◐│  ← permission (yellow), floats
│                                     │●│  ← working (green)
│                                     │○│  ← working (green)
│                                     ╰─┤
└──────────────────────────────────────┘
```

Key geometric rules:

- **Attached, not floating.** The capsule's right edge has zero corner
  radius and zero gap from the physical screen edge — there is no visible
  "outside" to the right of it, exactly the way a physical port or button
  on the chassis itself would look, not a window someone dragged there.
- **Left edge only is rounded**, with a continuous ("squircle") curve
  rather than a circular arc, matching modern OS corner treatment rather
  than a generic CSS `border-radius` circle.
- **Vertical position**: vertically centered on the display by default,
  biased slightly above center — this keeps it clear of the macOS menu bar
  height and the Windows taskbar (typically bottom), without hardcoding
  either. Exact offset is a platform capability concern (`OverlaySurface`),
  not a design constant baked into content.
- **Auto-height, capped.** Height grows with the number of visible glyphs
  (§18 defines the cap) and shrinks immediately when they're no longer
  relevant — it never reserves space for agents that aren't active.
- **No visible chrome at rest.** In the idle state the entire capsule may
  be reduced to a sliver only a few pixels wide — present enough to feel
  intentional if someone looks for it, invisible enough to never compete
  for attention.

---

## 4. Idle State

Zero active sessions, zero pending permissions, nothing needing attention.

```
▐   ← a slim, low-opacity vertical mark. No logo, no ring, no number.
```

This is the state the surface spends most of its life in. It communicates
exactly one thing — "Verge is here, nothing needs you" — and communicates
it by *not* drawing anything louder than that. There is no "Verge logo
badge" sitting idle; the product's own mark is not precious enough to
demand permanent screen space either. If the user wants more than "nothing
needs you," that's what deliberate interaction (§17) and the detail surface
are for.

---

## 5. Active State (Single Agent)

One tool has a live `ActivitySession` in `Working`.

```
     ╭───╮
     │ ◉ │   ← the tool's own logo, small, true color
     ╰───╯
       ┊     ← a hairline-thin green tint tracing the capsule's edge
              behind the logo — not a filled background, not a badge
```

- The logo is the tool's real, recognizable mark at fixed small size (§12).
- "Working" is communicated by the green edge tint plus a barely-perceptible
  motion cue (§16) — never by a text label. A user should recognize
  "something is running" the way they recognize a lit LED, not the way
  they read a status chip.
- No percentage, no session count badge, no text appears in this state
  unless the user deliberately expands (§17) — one active tool with nothing
  urgent to say does not need more than its own presence.

---

## 6. Multi-Agent State

Two or more tools have live sessions.

```
     ╭───╮
     │ ◉ │   Claude — working (green edge)
     ├───┤
     │ ◈ │   Codex — working (green edge)
     ╰───╯
```

```
     ╭───╮
     │ ◉ │
     ├───┤
     │ ◈ │
     ├───┤
     │▲ +2│  ← cap reached: 3rd glyph + compact "+N" for the rest
     ╰───╯
```

- Glyphs stack vertically in the capsule, tightest reasonable spacing, no
  per-item card, border, or background — the capsule itself is the only
  container.
- A visible cap (§18) exists on how many individual glyphs render before
  collapsing into a `+N` — activity count must never make the capsule grow
  without bound. Seven supported integrations does not mean seven rows;
  it means the capsule is exactly as tall as *currently relevant*
  information requires, capped.
- Ordering is priority-driven, not alphabetical or install-order: a tool
  waiting on permission floats to the top (§7); otherwise, most-recently-
  active first.

---

## 7. Permission State

An `ActivitySession` in `WaitingOnUser`.

```
     ╭───╮
     │ ◐ │   ← same logo, now ringed in a restrained amber/yellow tint
     ╰───╯     a single small tick mark on the ring communicates elapsed
                wait time (not a number, not a countdown, not a badge)
```

This is the single most important moment Verge exists to surface, and it
gets exactly one visual concession beyond the working state: it moves to
the top of the stack (§6) and its tint shifts to yellow. That is the entire
escalation. There is no popup, no OS notification, no sound. The design
goal, stated plainly in the brief, is a glance that produces the thought
*"Claude needs me"* — not a dialog that produces the thought *"something
demands my attention right now."* A permission wait is common and often
brief; treating it like an alarm every time would make the one signal this
product cannot afford to dilute (the actual, rare, "something is wrong")
indistinguishable from routine.

Elapsed wait time, if shown at all in the ambient view, is a *shape* (an
arc filling around the ring) rather than a number — numbers belong to the
expanded view (§17), not the glance.

---

## 8. Completed / Stopped State

A session transitions out of active work without user action — the task
finished, or the process ended.

```
     ╭───╮
     │ ● │   ← brief red tint, same logo
     ╰───╯
        │
        ▼  (after a short, fixed grace period)
     ▐         ← collapses back toward idle / the ambient summary
```

The red tint is intentionally transient: it exists to answer "did that
just finish?" for a few seconds to anyone glancing over, then the glyph
retires from the stack rather than lingering as a permanent "done" trophy.
Verge does not keep a visible ledger of finished sessions in the ambient
surface — that is what a dashboard would do.

**Domain note, flagged rather than resolved:** `core/src/domain/usage.rs`'s
current `ActivityState` enum has exactly three variants (`Working`,
`WaitingOnUser`, `RecentlyIdle`) and no distinct "completed/terminated"
value. This design's red state is a *presentation-layer* interpretation —
a session that was `Working` and is no longer reported at all, rendered
transiently before removal — not a claim that a fourth domain variant
already exists. Whether `ActivityState` eventually needs an explicit
`Completed`/`Terminated` variant (as opposed to inferring it from a
session's disappearance) is a real open question for whoever builds the
first `ActivitySource`, not decided by this document, and **no domain
change is made here** per this task's implementation boundary.

---

## 9. Usage Presentation

Visual first, numeric second — and numeric only on demand.

- **Ambient glance**: a thin partial-ring or edge-fill behind/around the
  active tool's logo, filling proportionally to the most-constrained
  `UsageWindow`'s fraction. No digits.
- **Expanded view** (§17): the same ring, now paired with the actual
  number (`73%`) and its `Fidelity`/`Recency` honesty exactly as the domain
  already models it — `92%` vs `~92% · 4 min ago` vs `Retrying · last known
  92%, 12 min ago` (`docs/PRODUCT_ARCHITECTURE.md` §6's rendering table is
  authoritative here; this document does not redefine it, only positions
  it).
- When `UsageReading` is a bare `Count` (no published limit — exactly
  Claude Code's real, empirically-confirmed case,
  `docs/design/claude-code-windows-local-state.md`), the ring has no
  meaningful "full" state to fill toward. Render it as a static neutral
  ring (identity only, no fill animation implying a ceiling that doesn't
  exist) with the count available only in the expanded view — never
  invent a denominator to make the ring look complete.
- When `Availability` is not `Available` (`Unauthenticated`, `Unsupported`,
  etc.), there is no ring to draw at all — the logo alone, undecorated,
  communicates "nothing to report," and the *reason* lives in the expanded
  view (§17), phrased in user language (§21), never as a missing/broken
  ring.

---

## 10. Limit Presentation

**One line, one window — never a list.** This is a direct instruction from
the brief and it also happens to be exactly what `core::domain::
project_ambient_state`'s existing "most-constrained window wins" projection
already computes (`core/src/domain/ambient_state.rs`) — the design and the
domain agree without either having to bend toward the other.

Expanded view shows: `Resets in 51 min` (or the equivalent honest phrasing
for aged/estimated data). It never shows a second, third, or fourth
competing limit line for the same tool, even if the underlying tool
technically exposes more than one window — the *domain* already picks the
single most-constrained one; the *UI* must not undo that discipline by
re-expanding it into a list for the sake of "showing more."

---

## 11. Session Presentation

- **Exactly one active session**: no numeral anywhere — the presence of
  the glyph itself already says "one."
- **More than one**: a small numeral badge at the glyph (`×3`), shown only
  in the expanded view or as a minimal corner mark in the glance view if
  it fits without adding a second line of text.
- **Idle/recent, no live session**: no session count is shown at all in
  the ambient view. "0 sessions" is not information worth a permanent
  pixel — its absence already communicates that.

---

## 12. Logo Treatment

- Use each tool's actual, recognizable brand mark. Never a homogenized
  "generic AI agent" glyph, never a Verge-invented replacement symbol.
- Fixed, small optical size across every tool (not scaled by usage,
  importance, or activity) so the *stack*, not any individual logo,
  communicates hierarchy.
- Rendered at a size and contrast that stays legible against the neutral
  material at typical display density — this is a real, testable
  constraint (§19), not just an aesthetic preference.
- Logos never carry the state tint themselves (no recoloring, no
  desaturating a brand mark to "match the theme") — the tint lives in the
  material *around* the logo (§13), never replaces the logo's own color.
- No logo is ever enlarged to "star" it, and the surface never becomes a
  scrollable logo gallery — see the density cap in §18.

---

## 13. Tool Color Treatment

```
Verge material  =  neutral, low-saturation, near-black translucent
Tool identity   =  the tool's own real brand color, on its own logo, unchanged
State signal    =  a restrained red / yellow / green tint, localized to
                   that one glyph's immediate surrounding only
```

The whole surface is never red, yellow, or green — only the specific glyph
in that state carries the tint, and only as a thin edge/ring treatment,
not a filled background. This keeps multiple simultaneous states (§6)
readable at a glance instead of turning the capsule into a traffic light.
Verge does not have its own accent color competing with tool identities;
its own material is deliberately colorless so the *tools'* colors and the
*state* tints remain the only chromatic information in the surface.

---

## 14. Liquid Glass Material System

Explicitly **not** conventional glassmorphism. The distinction the brief
draws is the whole point of this section:

| Conventional glassmorphism (rejected) | Verge's Liquid Glass material |
|---|---|
| Heavy, uniform backdrop blur | Minimal, localized blur — only enough to imply depth against whatever is behind it, not a frosted slab |
| Obvious 1–2px light-colored "glass border" | No visible border at all; edges are defined by a soft luminance falloff instead of a stroke |
| Large, generic drop shadow | No cast shadow, or an extremely soft, tight ambient occlusion at most — the surface should look *attached* (§3), and attached things don't float above a shadow |
| Flat, uniform translucent fill | Subtle vertical material gradient (very slightly denser near the screen edge, very slightly more translucent toward the inner edge) suggesting physical depth without a visible gradient band |
| Static appearance regardless of content behind it | A restrained specular highlight that shifts almost imperceptibly with content/scroll behind it — present, but never demonstrative |
| Glass as the star of the UI | Content (logo, tint, ring) stays visually dominant; the material is felt, not admired |

Concrete parameters for whoever implements this later (directional, not
literal pixel values — actual tuning happens against a real compositor,
per platform, the way the overlay-capability spike already established for
transparency mechanics):

- Base material: near-black, high translucency at rest, translucency
  decreasing slightly as more content (glyphs) is present, so a busier
  capsule reads as slightly more "solid" without a hard threshold.
- No flat color fill — always at least a two-stop vertical gradient at low
  contrast (a few percent luminance difference, not a visible band).
- Corner treatment: continuous curvature, not circular `border-radius`.
- One hairline highlight at most, on the capsule's outer (right, screen-
  edge-facing) side only, extremely low opacity — the "this has a physical
  edge" cue, not a decorative rim light.
- Never combine more than one attention-getting material effect at once
  (e.g., a permission tint pulse and a specular sheen animating
  simultaneously) — the brief's motion restraint (§16) applies to the
  material itself, not only to icons/text.

This is a rendering-technique specification for the eventual native
`ui/ambient/<os>` implementations, each of which will need its own
platform-appropriate technique (compositor blur APIs differ completely
between Win32/DWM, X11, and a future macOS/AppKit surface) — this document
fixes the *look*, not the API.

---

## 15. Typography

Directional reference taken from the inspiration image's two-tier
restraint, applied to a scale consistent with what this project's own
earlier ambient-surface mockups already converged on independently
(`docs/design/mockup-ambient-notch.html`, `mockup-macos-corner-strip.html`
— both landed on an ~11/14/18px, ~1.25–1.3-ratio scale after design-review
iteration):

| Tier | Size | Weight | Used for |
|---|---|---|---|
| Micro | ~11px | Regular | Meta text in the expanded view only (reset time, elapsed-wait phrasing) — never in the ambient glance |
| Body | ~14px | Semibold | The one primary word/value the expanded view needs (a tool name, a percentage) |
| Emphasis | ~18px | Semibold | Reserved, used sparingly — e.g. a session-count numeral if it needs to read clearly at a glance |

Rules:

- System fonts only, per OS (SF Pro / Segoe UI Variable / the Linux
  desktop's own UI font) — no bundled webfont, no "brand typeface."
- Numerals are tabular/fixed-width wherever a percentage or count can
  change between renders, so the capsule's width doesn't jitter as the
  number updates.
- No all-caps labels, no letter-spacing tricks to fake emphasis — weight
  and value contrast do that job (mirroring the inspiration image's own
  restraint).
- The ambient glance view (§4–§8) uses **no text at all** by default.
  Typography exists almost entirely in the expanded view (§17); this is
  itself a density rule (§18), not just a type-scale rule.

---

## 16. Motion Principles

> Motion communicates state. It does not decorate.

- **Presence/absence**: fade, not slide-and-bounce. A glyph appearing or
  leaving the stack crossfades over a short, fixed duration.
- **Expansion** (§17): the capsule's shape morphs smoothly (width/height
  eased, not spring-overshoot) — this is the one place a layout-affecting
  transition is justified, because it *is* the interaction, not
  decoration around it (a lesson already learned the hard way while
  building this project's own HTML design mockups, where an identical
  width/padding transition was reviewed and kept for exactly this reason:
  a single small, discretely-triggered element, not a performance
  concern).
- **State tint changes** (green → yellow → red) crossfade, they never
  snap or flash.
- **Working state**: at most a barely-perceptible breathing (a few percent
  opacity variation, if any motion is used at all) — not the saturated,
  looping pulse this project's own earlier mockup iteration correctly
  removed after design review (`docs/design/mockup-ambient-notch.html`'s
  glow-shadow removal is the precedent to hold this to).
- **Permission state**: exactly one gentle, non-repeating attention cue
  when it *begins* (e.g. a single soft scale-up-and-settle), never a
  continuous loop — a looping animation for something that might be
  pending for many minutes would itself become the visual noise this
  product exists to prevent.
- **Forbidden outright**: bouncing, spring overshoot, continuous looping
  attention effects, glowing/pulsating neon, any animation whose purpose
  is to look "alive" rather than to report a state change.
- **Respect OS "reduce motion."** Every transition above degrades to an
  instant (or near-instant crossfade-only) change when the platform
  signals a reduced-motion preference — this is a hard requirement, not a
  nice-to-have (§20).

---

## 17. Expansion Behavior

```
ambient glance  →  compact expansion  →  detail / settings (ui/detail, Tauri, not yet built)
```

Never:

```
tiny dashboard  →  larger dashboard  →  massive dashboard
```

- The ambient glance (§4–§8) is passive — it updates itself, the user
  never has to interact with it for it to do its job.
- **Compact expansion** happens only on deliberate interaction (hover or
  click, platform-appropriate) and stays attached to the right edge —
  it grows the same capsule leftward/wider, it does not spawn a separate
  floating window. It reveals, per active tool: session count (§11), the
  usage ring plus its number (§9), the one limit line (§10), permission
  state with elapsed time as a real value (§7), and nothing else.
- **Detail/settings** is the already-deferred `ui/detail` Tauri surface
  (`docs/adr/0004-shared-tauri-detail-surface.md`) — this document does
  not pull that work forward; it only confirms the compact expansion must
  not grow to try to replace it. If a user wants historical data, per-tool
  configuration, or account management, that lives in detail/settings,
  not in an ever-growing edge capsule.
- Closing the expansion (losing hover, clicking away, or an explicit
  dismiss) returns to the ambient glance without leaving any residual
  "recently expanded" visual trace.

---

## 18. Density Rules

- **Glyph cap**: a fixed, small maximum number of individual tool glyphs
  render in the stack (directionally 3–4) before the rest collapse into a
  single `+N` mark. The cap exists specifically so "Verge supports seven
  integrations" never becomes "the capsule is seven rows tall."
- **One line of state per glyph, maximum**, in the ambient glance — no
  glyph ever gets two stacked pieces of information in the passive view.
- **No permanent reserved space.** Sessions, permissions, and limits each
  occupy zero space when not relevant — this project's domain already
  treats absence-of-data as a first-class case (`Availability`,
  `docs/PRODUCT_ARCHITECTURE.md` §6); the density rule is simply the same
  discipline applied to layout instead of data.
- **More activity must never look proportionally noisier** — the `+N`
  mechanism (§6) is the release valve that keeps this true; if the cap is
  ever hit routinely in real use, that is a product signal to reconsider
  which tools genuinely need ambient presence, not a reason to raise the
  cap indefinitely.

---

## 19. Responsive / Multi-DPI Behavior

Grounded directly in what has already been empirically tested, not
aspirational:

- **Per-monitor DPI awareness is mandatory**, matching the pattern already
  proven necessary and working in `platform/windows/src/overlay.rs`
  (`SetProcessDpiAwarenessContext`/`DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`)
  — capsule dimensions are specified in logical units that the platform
  layer resolves to physical pixels per-monitor, never a single hardcoded
  physical-pixel constant assumed to be correct everywhere.
- **Multi-monitor placement is explicitly out of scope for this design
  pass**, matching `docs/STATUS.md`'s own honest status — neither the
  Windows nor the Linux/X11 vertical slice has multi-monitor evidence yet.
  This document specifies single-primary-display behavior only; a
  multi-monitor placement rule (which display, and whether it should
  follow the active/focused display) is a real future design decision,
  not silently assumed here.
- **Text and glyph sizes are defined in the same logical-unit space** as
  geometry, so a 150%-scaled display doesn't get a capsule with correctly
  scaled dimensions but blurry or mis-scaled icons inside it.
- **Vertical position must re-anchor on a resolution/DPI change** (a
  display hot-plug or scale-factor change) rather than persist a stale
  physical-pixel coordinate — this was flagged as `NOT TESTED` for the
  Windows overlay in `SPIKE_RESULTS.md` and remains an open verification
  item for the real implementation, not something this document can claim
  is solved.

---

## 20. Accessibility Considerations

- **Never rely on color alone.** Every state tint (§7, §8) is paired with
  a second, non-color signal: permission state also changes stack position
  (floats to top) and gets the one-time motion cue (§16); completed/stopped
  state is also transient-then-absent, not just red-then-gone. A user with
  a color-vision deficiency can still read every state from position and
  behavior alone.
- **Contrast**: state tints and logo rendering against the near-black
  material must meet at least a 3:1 contrast ratio for non-text UI
  (WCAG 2.x's non-text contrast guidance) — this is a concrete acceptance
  check for implementation, not just a color preference.
- **Respect reduced-motion OS settings** (§16) — this is treated as an
  accessibility requirement, not a stylistic option.
- **Hit targets**: even though the ambient glance is visually minimal, the
  interactive (hoverable/clickable) region for triggering expansion must
  meet normal platform minimum hit-target sizing — visual smallness must
  not become a literal small click target.
- **Native accessibility tree exposure** (screen reader name/state per
  glyph — e.g. "Claude, working" / "Codex, waiting for permission") is
  flagged here as a real future requirement for whichever platform
  `OverlaySurface` implementation ships this UI, since none of the current
  Windows or Linux/X11 implementations expose one yet
  (`docs/STATUS.md`) — not solved by this document, not silently ignored
  by it either.

---

## 21. What Must Never Appear in the UI

Implementation language, verbatim examples the brief called out and this
document extends slightly for completeness:

```
Auto-detected
Reading local state only
Local RPC port discovery
Process-table scraping
Read-only
Credential source / CredentialStore
Local state
Adapter / Provider / Capability
Availability::Unauthenticated (or any raw enum/type name)
Fidelity: Derived (raw domain vocabulary, unparaphrased)
```

Everything above is real, correct, load-bearing vocabulary *inside*
`core/domain` and this project's own docs — that is exactly why it must
never leak into the ambient surface. The domain's job is to be precise
internally; the UI's job is to translate that precision into something a
user reads as a fact about their tools, not a fact about Verge's own
implementation. (`docs/PRODUCT_ARCHITECTURE.md` §6's rendering table
already does this translation correctly for text — e.g. `Unauthenticated`
becomes "Needs sign-in in Claude Code" — this document's job is only to
make sure that same discipline extends to the ambient glance's visual
vocabulary, not just its expanded-view copy.)

Also never:

- A permanent roster of every supported tool, active or not (§6, §18).
- More than one competing limit/model line per tool (§10).
- Fabricated/placeholder usage numbers presented as if real, in production
  UI (mockup/prototype data is fine and already used elsewhere in
  `docs/design/*.html`, but must stay clearly synthetic and never ship as
  default UI state).
- A MacBook-notch shape, or notch-merging behavior, outside macOS — and
  arguably not assumed even on macOS by default, since this brief
  explicitly retires the notch as the product's identity in favor of the
  right-edge surface (§3).
- Top-center or centered-on-screen placement of the ambient surface.
- A full-screen overlay of any kind.
- Glassmorphism tropes: frosted rectangles, visible glass borders,
  oversized shadows, generic backdrop blur used as decoration rather than
  material (§14).
- Decorative gradients, neon glows, or bouncing/spring/looping motion used
  for anything other than a genuine, restrained state signal (§16).
- Giant headings or dashboard-style section titles anywhere in the ambient
  or compact-expansion views.

---

## 22. Example State Compositions

**(a) Idle — no active sessions, any OS with `FULL` ambient capability**

```
▐
```

**(b) One tool working**

```
╭───╮
│ ◉ │  (green edge tint, no text, no ring unless usage is Available)
╰───╯
```

**(c) Three tools, mixed states, one needs permission**

```
╭───╮
│ ◐ │  Claude — waiting for permission (yellow, floated to top)
├───┤
│ ◈ │  Codex — working (green)
├───┤
│ ▲ │  Cursor — working (green)
╰───╯
```

**(d) Seven tools supported, two active — the density rule in practice**

```
╭───╮
│ ◉ │  Claude — working
├───┤
│ ◈ │  Codex — working
╰───╯
```

*(The other five supported-but-inactive tools occupy zero pixels. "Seven
integrations" is a fact about the product, not a fact the ambient surface
is obligated to display.)*

**(e) `REDUCED` mode — e.g. a Linux/X11 desktop with no system tray, or a
desktop environment where the edge surface is available but degraded**

```
[tray icon: ◉]   ← same logo/tint language, compressed into whatever
                   single-glyph affordance the platform actually offers;
                   no fabricated edge capsule pretending FULL capability
                   exists where it doesn't
```

**(f) `ON-DEMAND` mode — e.g. GNOME/Wayland, per `SPIKE_RESULTS.md`'s own
still-open finding — no persistent ambient surface exists at all**

```
(nothing persistent on screen)

Opening the detail window shows the same visual language — logo, tint,
one ring, one limit line — inside an ordinary window instead of an edge
capsule. The user still recognizes it as Verge; they just have to open it
deliberately, and the product must say so plainly (per
`docs/PRODUCT_ARCHITECTURE.md` §15: "the telemetry works, but this desktop
environment cannot provide the ambient overlay") rather than looking
broken or silently absent.
```

---

## Design Self-Test

Answering the brief's own final checklist directly, against the
specification above:

- **Does the UI work if only one agent exists?** Yes — §5 is exactly one
  glyph, no stack, no `+N` machinery engaged.
- **Does it remain elegant with three agents active?** Yes — §6/§18's cap
  handles this before it becomes crowded; three is below the cap.
- **Does it remain calm with seven integrations supported?** Yes — §22(d)
  is the direct answer: supported ≠ visible.
- **Does it communicate "Claude needs permission" in one glance?** Yes —
  §7: yellow tint + top-of-stack position, no text required.
- **Does it communicate "Claude is working" without text?** Yes — §5:
  green edge tint + optional near-imperceptible motion, no label.
- **Does it communicate usage without becoming a chart?** Yes — §9: a
  single partial ring, numbers deferred to deliberate expansion.
- **Does the right-edge placement feel intentional?** That is the explicit
  goal of §3's "attached, not floating" geometry (flush edge, no margin,
  zero corner radius on the screen-facing side) — a real answer requires
  building it against a real compositor, exactly as the overlay-capability
  spike did for the *mechanics* of edge placement; this document specifies
  the intent precisely enough for that build to be judged against it.
- **Does the interface still make sense on Windows?** Yes — nothing here
  requires anything beyond what `platform/windows`'s already-validated
  `OverlaySurface` mechanism provides (§14, §19 reference it directly).
- **Does it still make sense on Ubuntu where the ambient surface may be
  unavailable?** Yes — §22(e)/(f) specify the `REDUCED`/`ON-DEMAND`
  degradations explicitly, using the same visual language rather than
  inventing a different one per mode.
- **Does the design remain coherent in `REDUCED` or `ON-DEMAND` mode?**
  Yes, by construction — §22(e)/(f) are part of this specification, not an
  afterthought bolted on after a `FULL`-mode-only design was finished.
- **Does it still look premium without relying on the MacBook notch?**
  Yes — §2 explicitly separates the inspiration's *design thinking* from
  its *geometry*, and §3 defines an edge-mounted identity that owes nothing
  to any specific hardware feature.

No answer above required revising the design; where a question exposed a
genuine open gap (multi-monitor behavior, §19; the domain's lack of an
explicit "completed" activity state, §8; native accessibility tree
exposure, §20), it is recorded as an explicit, named open item rather than
quietly assumed solved.
