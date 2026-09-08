# Status

Last updated: 2026-09-07 (Windows ambient surface redesign: right-edge
Liquid Glass capsule). A component is only marked `VALIDATED` if this
repository contains evidence (a passing test, a screenshot, a log) — not
because the architecture doc expects it to work.

| Component | Status | Notes |
|---|---|---|
| Architecture | VALIDATED | `docs/PRODUCT_ARCHITECTURE.md`, the overlay-capability spike (`D:\overlay-capability-spike\SPIKE_RESULTS.md`), and now a second independent tool+OS pair — see `docs/design/SECOND_VERTICAL_SLICE.md` |
| Ambient visual design | VALIDATED (design) | `docs/design/VERGE_AMBIENT_DESIGN.md` — the approved visual source of truth; implemented for Windows, see `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md` |
| Overlay capability (Windows) | VALIDATED | Right-edge Liquid Glass capsule, real per-pixel alpha, live-tested (compact, expanded-on-hover, idle, always-on-top over a maximized window); see `docs/design/evidence/ambient_capsule_compact.png`, `ambient_capsule_expanded.png`, `ambient_idle_sliver.png`, `ambient_topmost_over_notepad.png`, and `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md` for exactly what is and isn't covered |
| Overlay capability (Linux/X11, real WM) | VALIDATED | Production `platform/linux/x11` implemented and live-tested (Xephyr + metacity, not bare metal); see `docs/design/evidence/xephyr_clickthrough_typed_text.png` and `docs/design/SECOND_VERTICAL_SLICE.md` |
| Overlay capability (Sway/wlroots) | PROTOTYPE (spike only) | `SPIKE_RESULTS.md` §4; click delivery not live-confirmed (WSLg limitation, not protocol); no production implementation yet |
| Overlay capability (KDE/Wayland) | NOT STARTED | Never independently tested, spike or production |
| Overlay capability (GNOME/Wayland) | BLOCKED | Genuinely unresolved by the spike; must not be assumed `FULL` (see `SPIKE_RESULTS.md` §6, §8) |
| Core domain | VALIDATED | `core/` — 13 passing unit tests covering the "never invent usage" and capability-degradation invariants, now exercised by two structurally different tools (Claude Code's `Unauthenticated`, Codex's tool-inherent `Unsupported`) |
| Tool integration — Claude Code | VALIDATED (Windows, local-file signal only) | `tools/claude` — 7 passing tests (unit + contract); real local state discovered and documented in `docs/design/claude-code-windows-local-state.md`; official-endpoint (network) usage signal not built |
| Tool integration — Cursor | NOT STARTED | No real Cursor installation with usable local state found on this development machine — see `docs/design/claude-code-windows-local-state.md` |
| Tool integration — Codex | VALIDATED (Linux, local-file auth check only — no usage signal exists to validate) | `tools/codex` — 5 passing contract tests; real local state discovered and documented in `docs/design/codex-linux-local-state.md`; **confirmed no local usage/quota cache exists at all** — usage requires a live authenticated network call this adapter deliberately does not make |
| Windows platform | PROTOTYPE | `platform/windows` — `CredentialStore` (file-based, not Credential Manager — see design doc) and `OverlaySurface` (now a real per-pixel-alpha right-edge capsule, not color-key transparency) both implemented and live-tested; no `FileWatcher`, `ProcessInspector`, `SystemPresence`, `LoginRegistrar`, or `Updater` yet; multi-monitor placement still unverified (single-display hardware only) |
| macOS platform | NOT STARTED | |
| Linux/X11 platform | PROTOTYPE | `platform/linux/x11` — `CredentialStore` (file-based, covers Codex's default `File` auth-storage mode only, not `Keyring`/`Auto`) and `OverlaySurface` both implemented and live-tested; no `FileWatcher`, `SqliteReadOnlyOpener`, `ProcessInspector`, `SystemPresence`, `LoginRegistrar`, or `Updater` yet |
| Linux/Wayland platform | NOT STARTED | |
| `ui/ambient` (Windows) | VALIDATED | `ui/ambient/windows` — implements `docs/design/VERGE_AMBIENT_DESIGN.md` (right-edge capsule, hover-driven expansion, honest jargon-free copy, density-capped glyph stack); renders real `AmbientState` from the live Claude Code adapter; 8 passing unit tests. State tint (green/yellow/red) and session count are architecturally present but always `Neutral`/absent today — no `ActivitySource` exists for any tool yet, see `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`'s "Activity Gap" |
| `ui/ambient` (Linux/X11) | VALIDATED | `ui/ambient/linux-x11` — renders real `AmbientState` from the live Codex adapter; 2 passing unit tests on the pure formatting function; formatting logic is currently duplicated verbatim with the Windows shell, deliberately not abstracted yet (see `docs/design/SECOND_VERTICAL_SLICE.md`) |
| `ui/ambient` (macOS) | NOT STARTED | |
| `ui/detail` (Tauri) | NOT STARTED | Deliberately deferred — see `docs/adr/0004-shared-tauri-detail-surface.md` |
| Testing | PROTOTYPE | Domain + contract tests automated and passing (39 total, up from 33); capability tests are manual only — see `tests/capability/README.md` |
| Packaging | NOT STARTED | |

## What actually runs today

`cargo run -p verge-desktop --bin verge` (Windows) launches a real,
always-on-top, click-through, right-edge-flush vertical capsule — a near-
invisible sliver when idle, a badge with a neutral usage ring when a real
account is discoverable, expanding on hover to show live data read from
this machine's actual `~/.claude/stats-cache.json`, gated by a real (if
minimal) credential check against `~/.claude/.credentials.json`. See
`docs/design/VERGE_AMBIENT_IMPLEMENTATION.md` for exactly what is and
isn't backed by real data.

`cargo run -p verge-desktop --bin verge-linux-x11` (Linux/X11 only) launches
the equivalent overlay against a real X server, showing Codex's real
sign-in status read from `$CODEX_HOME/auth.json` — honestly reporting
`Unsupported` for usage rather than a number, since no local usage signal
exists for Codex at all (see `docs/design/codex-linux-local-state.md`).

Neither binary does anything yet for any other tool, any other OS, or any
settings/detail view.
