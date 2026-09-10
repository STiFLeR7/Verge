# Session awareness E2E review handoff

Date: 2026-09-08. Environment: Windows native Win32 overlay, 1920 x 1200 primary display, 125% DPI, Inter, 0.9 surface scale. Result: the executed scenarios below passed after fixing the footer painting mismatch discovered in this run. This is not a claim of exhaustive product coverage.

## Defect found and corrected

The preceding navigation change updated four hit regions but left three old painted labels (`Back`, previous-plus-count, `Next`). Native screenshots exposed this mismatch; prior hit-area unit tests were insufficient. The renderer now draws four matching elements: `Back`, previous arrow, noninteractive counter, next arrow. Painting and hit testing share `session_navigation_bounds`. This was a real implementation defect, not a user interaction mistake.

The initial ChatGPT harness also assumed Claude's two-row tooltip height; it now uses the fixture's correct one-row ChatGPT geometry. The live app was stopped during fixture runs to prevent overlapping topmost windows from contaminating screenshots. Capture explicitly uses physical DPI coordinates. A window-contract attempt was interrupted by external pointer movement; the retry passed. These interrupted/invalid runs are not counted as passes.

## Executed checks

| Scenario | Result | Evidence / scope |
| --- | --- | --- |
| Formatting | PASS | `cargo fmt --check` |
| Workspace | PASS | 65 tests; includes parsers, domain, presentation, native rendering/geometry and hit regions at four scales |
| Claude observer privacy contract | PASS | `node scripts/claude-signal.test.cjs`; synthetic metadata only |
| ChatGPT native navigation | PASS | Enter opens; right arrow advances; counter does nothing; left arrow reverses; both wrap; mouse Back and Escape restore quota view |
| Claude native navigation | PASS | Same scenario, separate brand run |
| Native permissions UI | PASS | Deny, Dismiss, Approve via full review, closing review denies |
| Real permission transport | PASS | Actual helper and native broker, synthetic process/session identity; approve, deny, dismiss, disconnect, terminated ancestor; mismatched request/session rejected |
| Window behavior | PASS on retry | Six expand/collapse cycles while source delays 1800 ms; right edge retained, click-through toggles, reminder does not steal focus, GDI handles 1 -> 1 |
| Native inactivity | PASS | Working-session fixture, no Verge interaction for 32 seconds: 6 px idle bar; hover reveals 442 px surface |
| Live Codex ingestion | PASS | Three recent sessions, all with reported models/context; no transcript output. This is observation, not a controlled model-change experiment |
| Live Claude ingestion | NOT AVAILABLE | Zero verified live Claude sessions during probe |

Navigation sends `WM_LBUTTONDOWN`/`WM_LBUTTONUP` to the isolated native fixture window and compares popup raster content. It is not hardware-mouse automation. Real pointer-based hover/click-through is exercised separately by the window contract. Permission tests never act on real user requests or execute a tool command.

## Reproduce

Run from repository root, with the actual Verge overlay stopped during fixture captures. Keep the pointer still during window tests. Each script closes its own fixture; session/window tests restore the pointer.

```powershell
cargo fmt --check
cargo test --workspace
node scripts/claude-signal.test.cjs
cargo build --release -p verge-platform-windows --example visual_states
powershell -NoProfile -ExecutionPolicy Bypass -File platform/windows/tests/session_ui_contract.ps1 -Brand ChatGPT
powershell -NoProfile -ExecutionPolicy Bypass -File platform/windows/tests/session_ui_contract.ps1 -Brand Claude
powershell -NoProfile -ExecutionPolicy Bypass -File platform/windows/tests/permission_ui_contract.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File platform/windows/tests/window_contract.ps1
cargo build -p verge-desktop --bin verge-claude-hook
cargo run -p verge-desktop --example permission_contract
cargo run -p verge-desktop --example session_inspection
```

The native inactivity check was a separate timed run of `visual_states sessions --open`: wait 32 seconds with pointer outside, assert window width <= 10, move pointer to idle bar, wait 800 ms and assert width > 100. The deterministic inactivity policy is also covered in workspace tests.

## Screenshots inspected

Synthetic data, captured from the actual renderer, not product usage:

- [ChatGPT session](evidence/e2e-chatgpt/first.png)
- [ChatGPT next session](evidence/e2e-chatgpt/next.png)
- [ChatGPT Back to quota](evidence/e2e-chatgpt/back.png)
- [Claude session](evidence/e2e-claude/first.png)

Each brand's evidence directory also contains counter, previous, wrap, wrap-next and Escape captures. Main visual check confirms separate Back and arrow controls, readable Inter text, session context distinct from account quota, and explicit missing-data labels.

## Review focus and limits

- Read `SESSION_INTELLIGENCE.md` for data semantics, provider research, privacy and architecture decisions. Later dated sections supersede earlier fixed-tail/capture limitations.
- Inspect `tools/codex/src/local.rs`: last-request total / capacity is Derived, not cumulative account usage; bounded backfill starts at 512 KiB, caps at 4 MiB. No model guessing beyond that cap. Stateless bounded rereads can still be costly with many large active files.
- Inspect `core/src/domain/session.rs`: freshness, unavailable/unknown/estimated signals, pressure policy and prioritization. Thresholds indicate fullness, not predicted failure.
- Inspect `platform/windows/src/overlay.rs`: permission precedence, four footer regions, session-selection identity persistence and inactivity behavior.
- Inspect Claude writer/parser and permission transport boundaries. No new Codex direct-approval channel is implemented.
- Native E2E captures were at 125% DPI only; 100/125/150/200% geometry/hit regions have automated rendering tests. Multi-monitor movement, screen readers, real provider compaction/model switching, reconnects over long periods and ChatGPT web/cloud conversations are not certified by this run.
- The repository contains extensive earlier uncommitted and untracked work. Review untracked files as well as `git diff`; there is no clean baseline commit for this full phase. No commit or publication was made.


## Independent Claude review follow-up

Verified and addressed the five reported findings. The shipped installer now registers the synchronous `PermissionRequest` helper alongside the passive observer, validates the sibling executable pair first, quotes paths, preserves unrelated hooks and is idempotent. Its output warns explicitly that unavailable Verge/timeout means deny. The current user's local settings were installed and checked: exactly one helper, timeout 130 seconds, debug executable pair running. A Claude restart/reload is still required; no live permission request was generated to prove Claude has reloaded that configuration.

Pi/Kilo qualification: the old Rust installer already emitted the dependency under `verge-observer-lib.mjs`, so installed plugins did not have the claimed missing-file condition. Source/package naming was nevertheless inconsistent. Renamed the source module to the installed/imported name, removed the test-only dependency alias, and added a test of the actual Rust installer layout. Existing observer files remain protected from overwrite.

Marked SIMPLIFIED_FEATURES as historical and added `CLAUDE_APPROVAL_SETUP.md` describing fail-closed operation, removal/recovery and the same-user threat boundary. A process ancestor walk and readable local session metadata are correlation, not proof against malicious same-user parent-PID spoofing. No security checks were removed.

Follow-up verification: fmt, 66 workspace tests, both Node suites, executable build and all five native helper/broker contract cases passed. Installer tests cover direct registration, repeated installation, preserved user hooks/status line, paths containing spaces, and missing-binary preflight with unchanged settings. These are source/install/transport checks, not a fresh manual Claude-host approval exercise.

## Latest portability E2E run

See [E2E_PORTABILITY_2026-09-10.md](E2E_PORTABILITY_2026-09-10.md) for the fresh Windows/Linux results, extended native navigation/inactivity coverage, interrupted pointer runs and remaining limits.
