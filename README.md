# Verge

A cross-platform ambient AI developer-awareness layer that shows how much
runway remains across the AI coding tools a developer uses, and whether an
agent is working or waiting — without becoming another coding assistant,
dashboard, task manager, or AI cockpit.

## What Verge is not

Not a dashboard, not an analytics platform, not a coding assistant, not a
system monitor, not a quota-webpage wrapper, not a task manager, not a
terminal frontend, not an "AI cockpit." See
`docs/PRODUCT_ARCHITECTURE.md` §1 for the full, deliberate list and why each
is rejected permanently, not just deferred.

## Architecture

```
Core Domain (core/)              — pure Rust: Tool, Account, UsageSnapshot,
                                    Fidelity, Recency, Availability,
                                    CapabilityProfile, AmbientState
        │
   ┌────┴────┐
Tool         Infrastructure
Adapters     Capabilities
(tools/*)    (platform/*)
   │             │
Claude Code   Windows: CredentialStore, OverlaySurface
   │             │
   └────┬────────┘
   Application (apps/desktop)
        │
   ┌────┴────┐
ui/ambient   ui/detail
(native,     (Tauri,
 per-OS)      not built yet)
```

Full reasoning: `docs/PRODUCT_ARCHITECTURE.md`. This is an **independent
implementation** — not a port, fork, or derivative of `codenotch` (the
macOS reference product this architecture was designed against). No source
from `codenotch` or from the disposable overlay-capability spike
(`D:\overlay-capability-spike\`) was copied into this repository; both were
read and learned from, not reused.

## Supported platforms today

Windows only. See `docs/STATUS.md` for the honest, per-component state —
including what's `NOT STARTED`, not just what works.

## Capability / degradation philosophy

> One product. Explicit capabilities. Honest degradation. Never a silent lie.

A `CapabilityProfile`, produced by the platform layer and consumed by
`core::application::select_product_mode`, is the only thing allowed to
decide whether the product runs in `FULL`, `REDUCED`, or `ON-DEMAND` mode —
never an ad hoc OS check scattered through UI code. Missing data is never
rendered as a real value: see `docs/adr/0005-honest-capability-degradation.md`.

## Privacy model

Verge reads only local state its own already-signed-in tools have already
written: OAuth/API tokens (read-only, never logged, never persisted to
Verge's own disk), local JSON/SQLite state (read-only). It never reads
keystrokes, clipboard contents, project source files, or credentials for a
tool it doesn't explicitly declare support for. No telemetry, by default,
ever. Full model: `docs/PRODUCT_ARCHITECTURE.md` §11.

## Current development status

First vertical slice complete and running: `cargo run -p verge-desktop`
(Windows) shows a real, always-on-top, click-through, borderless ambient
overlay rendering live local usage data read from this machine's actual
Claude Code installation. See `docs/STATUS.md` for exactly what that does
and doesn't cover, and `docs/design/claude-code-windows-local-state.md` for
how the Claude Code local-state discovery was made.

## Running

```
cargo test --workspace   # 23 tests: domain, contract, platform
cargo run -p verge-desktop
```

Requires a real Claude Code installation on Windows (i.e. a populated
`%USERPROFILE%\.claude\`) to show live data; otherwise the overlay honestly
reports `Needs sign-in` or `Nothing to meter yet` rather than a fabricated
number.
