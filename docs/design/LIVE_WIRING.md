# Live Claude activity and Codenotch geometry

Implemented and verified on Windows, 2026-09-08.

## Data flow

`ClaudeActivitySource` reads only bounded JSON session metadata from `~/.claude/sessions`. `WindowsProcessProbe` verifies that each PID is alive and its creation time matches the record's Windows FILETIME. Dead processes and reused PIDs are excluded. Legacy records without FILETIME require a start timestamp within five seconds. Unknown status stays unknown; unreadable or partial metadata does not become a fabricated zero-session count.

The desktop composition refreshes activity every second and caches usage for 60 seconds. Core selects waiting before working, explicit stopped/completed, idle and unknown. Windows presentation receives the verified count, reported state, session name/project and optional waiting reason. A real session remains visible without usage data. This does not read conversation transcripts or infer what tool call is running.

Reported `busy`/`active` means working, `waiting`/`blocked` means attention, `idle` means idle, and explicit `completed`/`stopped` means stopped. A process disappearing removes its session; no synthetic completion event is generated. Other provider integrations and settings remain outside this change.

## Dimensions

Reference: `D:/codenotch/Sources/DesignSystem/Design.swift`, `Sources/Notch/NotchLayout.swift` and its `docs/design/frame-124-hover-tooltip.png`. Frame measurements convert with **44/117** and round at the physical-pixel boundary.

| Element | Logical pixels |
|---|---:|
| Side rail | 69.948718 |
| Ring diameter | 44 |
| Track / progress stroke | 5.829060 / 3.008547 |
| Logo | 17.299145 |
| Flare / body corner | 38.735043 / 29.634188 |
| Top / bottom padding | 26.136752 / 18.841026 |
| Label gap / cell spacing | 10.116239 / 31.401709 |
| Context width / padding / corner | 225.641026 / 12.034188 / 18.615385 |
| Resting pill | 9.777778 × 78.974359 |

The connected expansion remains Verge's approved composition. Segoe UI replaces macOS SF; the numeric line box is explicitly 17 DIP, whereas Codenotch computes that box from SF metrics. Therefore font rasterization and font-dependent total height are not claimed pixel-identical to macOS. All listed frame measurements match the reference conversion.

## Verification

- `cargo test --workspace`: 45 passing tests, including PID reuse/death, status mapping, partial metadata, real Windows process timestamps, core-to-presentation activity precedence, geometry and renderer states.
- `cargo run -p verge-desktop --example activity_probe`: one verified real session, `RecentlyIdle`, one visible glyph. This diagnostic prints neither project names nor credentials.
- `platform/windows/tests/window_contract.ps1`: 120 DPI (125%), 87-pixel compact width, right-edge anchor, hover expansion/collapse, dynamic click-through, GDI object count 1 → 1.
- Renderer images cover 100%, 125%, 150% and 200%; they remain synthetic regression evidence.

The rebuilt live window also expanded from 87 to 369 physical pixels. [Live compact capture](evidence/live-wiring-compact.png) shows the verified count; [compact](evidence/codenotch-compact-125.png) and [expanded](evidence/codenotch-expanded-125.png) sheets show synthetic renderer states at the new dimensions. The desktop capture helper did not reliably capture the expanded region, so no expanded desktop screenshot is claimed.

Live working/waiting transitions depend on Claude publishing those statuses. Only the real idle state was observed during this validation; other mappings are tested with isolated metadata.
