# 0006 — Per-OS binary composition inside a shared `apps/desktop` package

## Context

The second vertical slice (Codex + Linux/X11) added a real second platform
target that needed its own composition root, alongside the first slice's
Windows one, inside the same `apps/desktop` package. The moment
`verge-ui-ambient-linux-x11` became a real dependency next to
`verge-ui-ambient-windows` in one `[dependencies]` table, an unconditional
`use` of a `cfg(windows)`-gated item on one side (and the symmetric case on
the other) broke compilation on whichever OS wasn't the target — a `Cargo`
package's dependency table is not itself conditional per-binary, only
`[target.'cfg(...)'.dependencies]` is, and that operates on the whole
package's build, not on an individual `[[bin]]`.

## Decision

For each OS-specific composition root:

1. The corresponding `ui/ambient/<os>` crate keeps its pure formatting logic
   (`render()`) ungated, and puts only the code that actually calls into a
   platform crate (`run_ambient_shell()`) behind `#[cfg(<os-predicate>)]`.
2. `apps/desktop` declares one `[[bin]]` per OS (`verge`, `verge-linux-x11`,
   ...), each with a `main*.rs` whose real `fn main()` is behind
   `#[cfg(<os-predicate>)]` and whose *other* platforms get a trivial stub
   `fn main()` behind the negation, so every binary target compiles
   everywhere even though only one is meant to run on any given machine.
3. Platform crates themselves (`platform/windows`, `platform/linux/x11`)
   already followed this pattern from the first slice (Win32-specific /
   X11-specific dependencies under `[target.'cfg(...)'.dependencies]`,
   OS-specific modules under `#[cfg(...)]`) — this ADR extends the same
   discipline up through `ui/ambient/*` and `apps/desktop`, rather than
   inventing a different mechanism at each layer.

## Reasoning

This keeps `cargo build --workspace` (and `cargo test --workspace`)
succeeding on every contributor's machine regardless of which OS they're
on, without needing cross-compilation toolchains or CI matrix gymnastics
just to type-check code nobody on that machine can run anyway. It also
keeps each OS's pure logic (the formatting function) unit-testable
everywhere, which matters more as more platforms are added — a bug in
`render()`'s formatting should be catchable in CI on any single OS, not
require a Linux runner to test a Windows-adjacent codepath.

## Consequences

Every future OS added to `ui/ambient/` and `apps/desktop` follows this same
three-part pattern. A reviewer should treat an *unconditional* cross-OS
`use` inside `ui/ambient/*` or a bare `fn main()` in `apps/desktop` (not
split into a real + stub pair) as a defect, not a style choice.

## Rejected alternatives

Cargo workspaces/features to conditionally exclude a member entirely per
OS — rejected as heavier than necessary: the actual problem was narrow (one
function per crate, one `fn main` per binary), and `cfg` attributes solve it
at exactly that granularity without restructuring the workspace.

## 2026-09-10 portability update

`apps/desktop` now uses target-specific native dependencies. `verge` dispatches by target OS; the old Linux binary remains an alias. Pure presentation lives in `ui/ambient/shared`. Linux predicates explicitly use `target_os = "linux"`, not `unix`. macOS uses a minimal AppKit Swift shell and the Rust `verge-state` presentation helper, consistent with ADR 0003. See `docs/design/PORTABILITY.md` for current capability gaps.
