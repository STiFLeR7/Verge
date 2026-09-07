# 0001 — Rust for `core/` (and the whole native surface)

## Context

`core/` must express the domain (Tool, Account, UsageSnapshot, Availability,
CapabilityProfile, ...) without depending on any OS API, vendor SDK, or UI
framework, and must be usable, unchanged, from three different native
`ui/ambient/` shells plus one shared Tauri detail surface. It also needs to
talk to OS-native credential stores, SQLite, and raw window/process APIs on
three operating systems eventually.

## Decision

`core/`, every `tools/*` adapter, and every `platform/*` implementation are
Rust.

## Reasoning

- Mature, memory-safe bindings exist for Win32, X11, and Wayland
  (`windows-rs`, `x11rb`, `smithay-client-toolkit`) — all three were
  directly validated by the overlay-capability spike before this decision
  was acted on (`D:\overlay-capability-spike\SPIKE_RESULTS.md`).
- No GC pause or idle-memory concern for a process meant to run permanently
  in the background.
- Tauri's backend is also Rust, so `ui/detail` consumes `core/` with zero
  FFI boundary.

## Consequences

- Every platform-specific implementation (`platform/windows` today) is Rust
  calling raw Win32 via `windows-rs`, which is more verbose than a
  higher-level toolkit — accepted, because `docs/PRODUCT_ARCHITECTURE.md`
  §14 explicitly bounds this cost to the *thin* `ui/ambient/` shells, not the
  whole application.

## Rejected alternatives

Qt (single-toolkit, C++) and Electron/Flutter were evaluated in
`docs/PRODUCT_ARCHITECTURE.md` §14 and rejected as the *primary* path,
mainly on Linux/Wayland capability grounds documented there. Qt remains a
logged fallback (§22, open decision 4) if the native-per-OS maintenance cost
proves too high — not decided now.
