# Status

Current entry points: [README](../README.md), [platform matrix](design/PORTABILITY.md), and [latest E2E report](design/E2E_PORTABILITY_2026-09-10.md). The dated sections below are a development log; later entries supersede earlier claims and test counts.

## Session intelligence slice — 2026-09-08

The current Windows app includes Inter, activity rings, compact direct-approval
controls, 30-second inactivity collapse, and real Claude/Codex quota sources.
These supersede the historical records below.

Session intelligence now extends verified Claude sessions with optional model
and context metadata from the existing status-line observer. The tooltip's
Sessions link opens one session at a time, ordered by attention, with Back/Next
navigation. Context is distinct from quota; stale, absent and estimated readings
are explicit. No average or inferred context capacity is used.

Research, capability matrix, architecture discrepancies and verification:
[SESSION_INTELLIGENCE.md](design/SESSION_INTELLIGENCE.md).
No live Claude process was verified during this pass, so live Claude context
values remain unvalidated; the real application and native synthetic session
navigation were visually inspected. Cursor/Codex context adapters are deferred.



## Current simplified rail — 2026-09-08

Claude session counts, both official status-line quota windows, reset times, threshold reminders and event-based activity tints are implemented. The live probe observed 2 sessions, 32% session usage and 65% weekly usage. All 48 workspace tests and the Node bridge/installer checks pass. The native reminder revealed automatically without taking focus. See [SIMPLIFIED_FEATURES.md](design/SIMPLIFIED_FEATURES.md) for scope, sources and verification limits. Earlier records below are historical.

## Previous implementation record



Last updated: 2026-09-08 (Windows ambient surface visual-system pass:

centralized design tokens, a real percentage-fill usage ring, and a smooth

expand/collapse morph — see `docs/design/VERGE_DESIGN_SYSTEM.md`). A

component is only marked `VALIDATED` if this repository contains evidence

(a passing test, a screenshot, a log) — not because the architecture doc

expects it to work.



`cargo build`/`cargo test --workspace` are green (39/39 tests).

`docs/design/evidence/ambient_capsule_compact.png` and

`ambient_capsule_expanded.png` have been re-captured against the rebuilt

binary (real Claude Code data, real cursor-driven hover-to-expand) and

confirm the surface still renders correctly after the tokens/motion/ring

changes.



| Component | Status | Notes |

|---|---|---|

| Architecture | VALIDATED | `docs/PRODUCT_ARCHITECTURE.md`, the overlay-capability spike (`D:\overlay-capability-spike\SPIKE_RESULTS.md`), and now a second independent tool+OS pair — see `docs/design/SECOND_VERTICAL_SLICE.md` |

| Ambient visual design | VALIDATED (design) | `docs/design/VERGE_AMBIENT_DESIGN.md` — the approved visual source of truth; implemented for Windows, see `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`; measurable token system in `docs/design/VERGE_DESIGN_SYSTEM.md` |

| Overlay capability (Windows) | VALIDATED | Right-edge Liquid Glass capsule, real per-pixel alpha, live-tested (compact, expanded-on-hover with the new eased morph, idle, always-on-top over a maximized window); see `docs/design/evidence/ambient_capsule_compact.png`, `ambient_capsule_expanded.png`, `ambient_idle_sliver.png`, `ambient_topmost_over_notepad.png` |

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


### Codex / ChatGPT session intelligence (2026-09-08)
Local Codex rollout metadata now feeds the shared session summary/drilldown: reported model, workspace basename, derived context capacity share and existing freshness/pressure prioritization. Real positive ingestion verified; 63 workspace tests and fmt pass. Bounded-tail gaps stay unknown. ChatGPT web/cloud sessions and Codex direct approvals are not claimed. Live layered-window capture was inconclusive. See `docs/design/SESSION_INTELLIGENCE.md` extension for evidence and limits.

Debug follow-up: long Codex turns recover model/workspace via bounded metadata backfill (512 KiB start, 4 MiB maximum). Fixed the test capture DPI mismatch at 125%; actual ChatGPT drilldown visually verified. 64 workspace tests, fmt, build and native navigation test pass.

E2E review pass (2026-09-08): fixed stale painted footer labels discovered by native screenshots. Both brands pass arrow/counter/Back/Escape scenarios; 65 workspace tests, native permission UI and real helper/broker contracts, window hover/focus/GDI checks, and timed idle collapse/reveal pass. See `docs/design/E2E_REVIEW_HANDOFF.md` for reproduction, evidence and limits.

Claude review follow-up: real PermissionRequest gate now registered by installer and applied locally; offline-deny behavior and same-user trust boundary documented in `docs/design/CLAUDE_APPROVAL_SETUP.md`. Pi/Kilo source and installed module naming aligned; actual install-layout test added. 66 workspace tests, fmt, both Node suites and native permission transport pass. Reload Claude to pick up the hook.

## 2026-09-10: portability foundation

Shared native presentation extracted without changing Windows rendering. Added portable local session composition, corrected Linux-only cfg boundaries, AppKit baseline and host packaging/CI. Windows portable archive built locally. Rust Linux/macOS cross-checks pass; native Swift and Unix UI execution remain unverified. See [PORTABILITY.md](design/PORTABILITY.md).

### Native Linux interaction follow-up

Added provider/session navigation, identity-stable selection, background metadata reads, 30-second inactivity collapse and waiting-state wake. Ubuntu 24.04 WSL workspace tests pass; the release archive executable passed the Xvfb smoke test with two isolated working Codex fixtures (collapse, hover reveal, no focus theft). Windows workspace tests and Apple Silicon Rust checks pass. macOS session identity/counter/wake fixes are implemented but AppKit remains uncompiled on this Windows host. `dist/verge-linux-x86_64.tar.gz` and the refreshed Windows ZIP were built.

### Repository documentation refresh

Updated README and product scope to match current Windows/Linux work and experimental macOS status. Added CONTRIBUTING, SECURITY, CHANGELOG, a documentation index, troubleshooting, the previously missing architecture overview, and GitHub issue/PR templates. Existing license terms are unchanged.
