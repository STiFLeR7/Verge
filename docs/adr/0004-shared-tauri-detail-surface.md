# 0004 — Shared Tauri `ui/detail/` surface (not yet built)

## Context

`ExpandableSurface`/`Settings` never need always-on-top, click-through, or
screen-edge positioning — the properties that forced `ui/ambient` to be
native-per-OS. Building it three times anyway would waste the one part of
the technology evaluation where a cross-platform framework's documented
weaknesses (Linux/Wayland `alwaysOnTop`, click-through) are irrelevant.

## Decision

`ui/detail/` will be one Tauri application, consuming `core/` directly
(same language, no FFI), when it is built. It is explicitly **not part of
this repository's first vertical slice** — see
`docs/PRODUCT_ARCHITECTURE.md` §16 step 9, "deliberately late."

## Reasoning

Tauri's real advantages (fast iteration, small footprint relative to
Electron, a built-in updater plugin) are pure upside for a surface that
never touches window-manager cooperation, with none of the downside that
ruled it out for `ui/ambient`.

## Consequences

No settings UI, no popover detail view exists yet in this repository. The
vertical slice's only observable output is the ambient overlay itself. This
is intentional scope, not an oversight — see `docs/STATUS.md`.

## Rejected alternatives

Building `ui/detail` alongside `ui/ambient` in this first pass — rejected
because it would spend effort on the least-risky, most-conventional part of
the system before the riskiest unknowns (per-OS overlay mechanics, real tool
local-state discovery) were even proven once.
