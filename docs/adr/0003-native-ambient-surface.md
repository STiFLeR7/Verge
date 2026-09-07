# 0003 — Native-per-OS `ui/ambient/` surface

## Context

The ambient overlay needs borderless rendering, real transparency,
always-on-top, and click-through. `docs/PRODUCT_ARCHITECTURE.md` §14 found
Tauri and Electron both document `alwaysOnTop` as unsupported on
Linux/Wayland, and neither has a reliable click-through story even on X11.

## Decision

`ui/ambient/<os>` is a thin, native, per-OS shell built directly against the
OS's real windowing API: `windows-rs` on Windows (this repository's only
implemented target so far), `x11rb` and `smithay-client-toolkit` on Linux
when built, AppKit-via-Rust-interop (or a minimal Swift shim) on macOS when
built. Each shell's only job is render + refresh — no tool logic, no
credential access, no HTTP.

## Reasoning

Confirmed empirically, not just planned: the overlay-capability spike proved
this mechanism works on Windows and Linux/X11-with-a-real-WM end-to-end
(`SPIKE_RESULTS.md` §2, §3, §10), and this repository's own vertical slice
now proves the *production* reimplementation of the Windows path renders
real `AmbientState` data from a real tool adapter
(`docs/design/claude-code-windows-local-state.md`; evidence:
`docs/design/evidence/vertical_slice_overlay_crop.png`).

## Consequences

Three platform implementations to maintain instead of one, bounded
deliberately to the smallest possible surface (`ui/ambient/windows/src/lib.rs`
is ~70 lines; the Win32 mechanics live in `platform/windows`, reusable by
any future Windows UI, not just this shell).

## A genuine correction found during this implementation

The spike's Windows design assumed `WS_EX_TRANSPARENT` could be part of the
window's initial `CreateWindowExW` style. Building the production version
surfaced a real bug: combined with `WS_POPUP` at creation time, Windows
silently produced a zero-size window at the origin. Fix: create the window
without `WS_EX_TRANSPARENT`, then add it via `SetWindowLongPtrW` immediately
after creation. A second, unrelated bug was also found and fixed in the same
pass: `SetWindowPos(..., 0, 0, 0, 0, SWP_NOACTIVATE)` without
`SWP_NOMOVE | SWP_NOSIZE` collapses the window on every re-assertion of
top-most, not just at creation. Both are documented inline in
`platform/windows/src/overlay.rs` where they were fixed.

## Rejected alternatives

Single-toolkit Qt (`layer-shell-qt`) remains logged as a fallback if the
native-per-OS Linux implementations (not yet built) prove disproportionately
costly — not decided now, no new evidence for or against from this slice.
