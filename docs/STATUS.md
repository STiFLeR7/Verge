# Status

Last updated: 2026-09-07. A component is only marked `VALIDATED` if this
repository contains evidence (a passing test, a screenshot, a log) — not
because the architecture doc expects it to work.

| Component | Status | Notes |
|---|---|---|
| Architecture | VALIDATED | `docs/PRODUCT_ARCHITECTURE.md`, plus the disposable overlay-capability spike's empirical results (`D:\overlay-capability-spike\SPIKE_RESULTS.md`) |
| Overlay capability (Windows) | VALIDATED | Spike + this repo's own production reimplementation, live-tested; see `docs/design/evidence/vertical_slice_overlay_crop.png` |
| Overlay capability (Linux/X11, real WM) | VALIDATED (spike only) | `SPIKE_RESULTS.md` §3b; no production `platform/linux/x11` implementation exists in this repo yet |
| Overlay capability (Sway/wlroots) | PROTOTYPE (spike only) | `SPIKE_RESULTS.md` §4; click delivery not live-confirmed (WSLg limitation, not protocol); no production implementation yet |
| Overlay capability (KDE/Wayland) | NOT STARTED | Never independently tested, spike or production |
| Overlay capability (GNOME/Wayland) | BLOCKED | Genuinely unresolved by the spike; must not be assumed `FULL` (see `SPIKE_RESULTS.md` §6, §8) |
| Core domain | VALIDATED | `core/` — 12 passing unit tests covering the "never invent usage" and capability-degradation invariants |
| Tool integration — Claude Code | VALIDATED (Windows, local-file signal only) | `tools/claude` — 7 passing tests (unit + contract); real local state discovered and documented in `docs/design/claude-code-windows-local-state.md`; official-endpoint (network) usage signal not built |
| Tool integration — Cursor | NOT STARTED | No real Cursor installation with usable local state found on this development machine — see `docs/design/claude-code-windows-local-state.md` |
| Tool integration — Codex | NOT STARTED | Present and richly instrumented on this machine — see `docs/design/claude-code-windows-local-state.md`; strong candidate for the next tool adapter, not built this pass |
| Windows platform | PROTOTYPE | `platform/windows` — `CredentialStore` (file-based, not Credential Manager — see design doc) and `OverlaySurface` both implemented and live-tested; no `FileWatcher`, `ProcessInspector`, `SystemPresence`, `LoginRegistrar`, or `Updater` yet |
| macOS platform | NOT STARTED | |
| Linux/X11 platform | NOT STARTED | |
| Linux/Wayland platform | NOT STARTED | |
| `ui/ambient` (Windows) | VALIDATED | `ui/ambient/windows` — renders real `AmbientState` from the live Claude Code adapter; 2 passing unit tests on the pure formatting function |
| `ui/ambient` (macOS/Linux) | NOT STARTED | |
| `ui/detail` (Tauri) | NOT STARTED | Deliberately deferred — see `docs/adr/0004-shared-tauri-detail-surface.md` |
| Testing | PROTOTYPE | Domain + contract tests automated and passing (23 total); capability tests are manual only — see `tests/capability/README.md` |
| Packaging | NOT STARTED | |

## What actually runs today

`cargo run -p verge-desktop` (Windows only) launches a real, always-on-top,
click-through, borderless overlay anchored to the bottom-right of the
primary display, showing live data read from this machine's actual
`~/.claude/stats-cache.json`, gated by a real (if minimal) credential check
against `~/.claude/.credentials.json`. It does not yet do anything for any
other tool, any other OS, or any settings/detail view.
