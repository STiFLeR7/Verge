# Session intelligence — evidence and implementation decision

2026-09-08. Research recorded before implementation. Windows / Claude is the first slice. This document distinguishes confirmed interfaces from locally observed signals and product-derived behavior.

## 1. Product problem
Developers need to identify the session waiting for them or nearing its context capacity without inspecting every terminal. Verge remains an ambient instrument.

## 2. Why counts are insufficient
Five sessions can include one blocked session and four uneventful ones. A count and account quota cannot identify the exception. Averages conceal the fullest context window.

## 3. Separate concepts
Tool identifies the product. Account owns quota; the existing Account is a local discovery scope, not verified universal account identity. Session owns current work. Model belongs to a session and can change. Context measures that session's current input footprint. Activity and pending approval are independent of context. Never sum context and quota.

## 4. Smallest domain
Extend ActivitySession with optional SessionIntelligence rather than inventing a competing session registry. ModelIdentity holds reported ID/display name. ContextUsage distinguishes unknown, unavailable and available observations; available observations carry fraction, optional reported capacity, timestamp and existing Fidelity. Add explicit disconnected activity for verified connection loss. Session IDs are scoped to their enclosing account/tool; Claude identities include the reported session identity and verified process creation time to prevent PID reuse. Duplicate observations are reduced deterministically within that scope.

## 5. Context pressure
Derived policy: below 80% = normal, 80–89% = high, 90%+ = near capacity. These are conservative inspection thresholds, NOT vendor failure or compaction boundaries. Auto-compaction can safely reduce pressure. No claim of imminent failure, no forecasting. Unknown/unavailable/stale/estimated observations do not produce trusted pressure. Estimated values remain visibly approximate in detail. No averages; count high sessions and select the highest-priority session.

## 6. Attention
Derive ordering rather than store another lifecycle enum: permission/waiting first, recent failure next, trusted high context on an active session next, then other work and unknown/idle. Within context attention, fullest first; ties stable. Completed/disconnected sessions never generate context alerts. Context does not recolor the existing activity ring or defeat the user's 30-second inactivity collapse. Pending permissions retain their override.

## 7. Reliability
CONFIRMED maps to Official vendor observations; DERIVED is computed projection; ESTIMATED stays Estimated; UNAVAILABLE is a known missing capability/reading; UNKNOWN means not established. Use timestamp plus the existing Recency classifier (120-second trust window), orthogonal to Fidelity. Retain last-known-good context on a transient parse/read failure without advancing its timestamp; visibly age it. Explicit null clears the current context claim. Invalid or future readings never become zero or live. A refreshed status line observes the vendor's last reported context, not a new model response.

## 8. Capability matrix

| Signal | Claude Code | Cursor | Codex |
|---|---|---|---|
| Session / workspace | CONFIRMED docs + local session metadata | CONFIRMED hooks; locally unavailable | CONFIRMED protocol; existing local rollouts |
| Model / model change | CONFIRMED status-line model fields | CONFIRMED hook model fields, version-dependent | CONFIRMED turn-context model in protocol |
| Context / capacity | CONFIRMED status-line percentage / capacity, nullable | CONFIRMED at preCompact; continuous sampling UNKNOWN | CONFIRMED token fields; exact local context projection remains research |
| Compaction / reset | Documented hooks and nullable post-compaction context; not inferred from drops | CONFIRMED preCompact event | Protocol compaction events; not implemented here |
| Working / end / error | Existing real hooks and verified process | Documented hooks; not locally tested | Existing task events; silence is not proof of idle |
| Pending approval | Existing request/session-bound pipe | Hook checks do not prove a pending interactive request | App-server waitingOnApproval; access to another running client's stream UNKNOWN |
| Start / connection | Verified process start is DERIVED session approximation | Session hooks; no local host | Thread metadata vs process liveness distinct |
| Quota | Existing account windows | Not investigated beyond existing scope | Existing Codex rate limits; separate from context |

## 9. Claude findings
[Official status-line documentation](https://code.claude.com/docs/en/statusline) exposes session ID, optional session name, model ID/name, workspace, input-context percentage and capacity. Nullable values occur before a response and after compaction. Use the published percentage rather than cumulative token totals. The existing local status-line script already consumes model/context fields and calls Verge's observer; no new hook installation or transcript reader is required. Local session JSON contains sessionId, PID, procStart, cwd and name. Current versions differ in available activity fields. The observer will whitelist metadata into a separate per-session file; the adapter joins only to verified sessions.

## 10. Cursor findings
[Official hooks](https://cursor.com/docs/hooks) document conversation/generation IDs, workspace roots and model fields. preCompact reports context percentage/tokens/capacity only when compaction occurs. stop/sessionEnd provide completion, aborted/error and closure information. These are confirmed documented capabilities, not proof of local continuous context availability. No Cursor executable was found at the known user installation path; existing status also reports no validated local integration. Do not read workspace databases or enable new permission hooks in this slice.

## 11. Codex findings
The [official protocol source](https://github.com/openai/codex/blob/main/codex-rs/protocol/src/protocol.rs) distinguishes total_token_usage, last_token_usage and model_context_window; TurnContextItem carries model and workspace. One bounded local tail confirmed those token metadata keys, without outputting transcript content. Lifetime total tokens are not current context. The [app-server interface](https://learn.chatgpt.com/docs/app-server) exposes thread status/token updates and waitingOnApproval, but attaching to another desktop/CLI client's live stream is not established. Existing file-based activity/quotas remain supported; no Codex context percentage is invented here.

## 12. Actually observable now
Claude metadata files plus installed status-line observer provide the real slice. ProcessProbe already verifies process creation, independently of tool parsing. Current account quota continues unchanged. Capture live new metadata after wiring to establish which model/context fields this installed version actually supplies.

## 13. Unknowns and document discrepancies
`docs/PRODUCT_ARCHITECTURE.md` is absent, including from repository file search. ADRs 0001–0006 establish the native/core/adapter seams and degradation rules; those are preserved. PRODUCT.md and STATUS.md describe obsolete Claude-only/no-direct-approval behavior. VERGE_AMBIENT_DESIGN.md calls itself unimplemented and specifies translucent material, old colors and no numeric rail; DESIGN.md and subsequent explicit user changes establish opaque black, activity rings, quota captions, Inter, permission replacement and inactivity collapse. Preserve the latter, record this discrepancy rather than redesigning. Cross-account identity, context-to-failure prediction, continuous Cursor context and independent Codex connection status remain unknown.

## 14. Ambient hierarchy
Keep the same logos, activity colors and session badge. No model tiles or extra context ring. On inspection, a compact session summary identifies high-context or waiting sessions; prioritize attention before truncating the tool cap.

## 15. Drilldown hierarchy
Reuse the existing native tooltip and input geometry: account usage remains the default; a Sessions affordance opens one session at a time with its real label, model, activity and explicitly named Context reading. Next/previous select sessions; Back restores account usage. Permission replacement always takes precedence. No new Tauri browser, history or scrollable dashboard.

## 16. UX examples (synthetic only)
Five working sessions, one waiting and one at 91%: show the waiting session first on inspection and indicate one near capacity. A session with absent context says Context not reported, never 0% or normal. A stale 91% reading says Last reported context 91%, without an active pressure warning. A model change replaces the model label; no model history is stored.

## 17. Security
No credentials, transcript reads, source contents, input recording, clipboard reads or telemetry added. Per-session metadata is bounded and validated; filenames accept only safe session IDs. Store only model identity, context observation and a workspace basename (not full workspace contents). Existing permission service remains sole authority: exact request/session/lifecycle checks, arming delay and full review remain untouched. Session browsing grants no permission.

## 18. Architecture impact
Core owns pure context/attention projection. Claude observer whitelists official fields; Claude Rust adapter interprets its own files with stdlib I/O and injected ProcessProbe. Windows draws generic presentation data and handles input only. No vendor parsing in Win32. The composition root continues using ActivitySource; other providers default to no intelligence.

## 19. Recommended implementation / verification
One Claude/Windows slice, pure domain tests, synthetic parser contracts, observer privacy tests, live metadata inspection, native screenshot, permission regression checks and full workspace fmt/tests. Preserve unknown instead of the existing Codex silence-to-idle inference as a minimal honesty correction. Document implementation evidence below after validation.

## 20. Non-goals
No all-provider rollout, Linux work, account merging, model catalog, inferred context ceilings, task-name generation, raw session IDs in UI, transcript mining, historical charts, telemetry, cloud, packaging or unrelated visual changes. No persistent context history or generic signal framework.

## Implementation and verification result

- Implemented ContextUsage, ContextPressure, ModelIdentity and optional SessionIntelligence on ActivitySession. Attention is a pure priority function, not another stored state machine. Disconnected is explicit in the domain; verified dead Claude processes are removed from the active set rather than called idle.
- Claude context joins use reported session identity plus PID and verified process creation time. Duplicate records collapse; a reused tool session ID across live processes suppresses ambiguous intelligence. New observer files contain only session ID, observation time, model identity and context percentage/capacity. The existing session metadata supplies the displayed name; no new workspace contents are collected.
- Malformed context preserves last-known-good data without refreshing its timestamp. Explicit null clears it. Model changes replace the displayed model. No model family/catalog or context history is added.
- Windows uses one Sessions footer and one session page with Back / previous / next; keyboard Enter, arrows and Escape are supported when input is delivered to the native surface. Pending permissions bypass session navigation and retain the existing arming/full-review decision path. Pending Claude requests are no longer lost when the normal tool cap/filter excludes Claude.
- Pressure policy is 80% high / 90% near capacity, fresh non-estimated observations only. Waiting outranks recent failure, which outranks context pressure, which outranks ordinary work. Stable per-session ordering avoids averages. Background context does not wake the surface after inactivity; approvals still do.
- Codex and generic observer silence now becomes Unknown, not inferred idle. Detection-only running apps are also Unknown; their legacy app-presence records are not validated conversation counts. Only Claude's new drilldown claims verified sessions.

### Checks and environment

Windows 11, 1920×1200 primary display at 125% scaling, Rust 1.98.1. `cargo fmt --check`, `cargo test --workspace` (**62 tests**) and `node scripts/claude-signal.test.cjs` pass. Domain/parser coverage includes one/many sessions, multiple high contexts, waiting alongside pressure, missing/unknown/stale/estimated values, missing model, completed/disconnected sessions, duplicate IDs, identity collisions and last-known-good preservation. Native raster fixtures also exercise 100%, 125%, 150% and 200%.

`platform/windows/tests/session_ui_contract.ps1` passed native Enter / Next / Back transitions and compared the tooltip raster before/after (synthetic sessions). Images were visually inspected: [account overview](evidence/session-overview-fixture.png), [context detail](evidence/session-detail-fixture.png), [unknown context](evidence/session-next-fixture.png). `permission_ui_contract.ps1` passed Deny, Dismiss, Approve through full review and ReviewClose; it only uses isolated requests, never real approvals.

The real application was rebuilt and visually inspected with genuine Codex usage and Claude's last reported quota. `cargo run -p verge-desktop --example session_inspection` found **zero verified live Claude sessions**. No `claude-context-*.json` had arrived from the real status-line source during inspection. Therefore the live empty case is verified, while a positive live Claude model/context observation remains **not verified**. Synthetic images above are not evidence of live Claude telemetry. The already-installed observer imports the updated module automatically on the next genuine status-line invocation; no fabricated event was injected into production.

### Boundary audit and remaining risks

No new raw OS calls in core or Claude context parsing; no tool schema parsing in the native session view. Existing ProcessProbe provides liveness and PID-reuse protection. Existing account quotas and permission-service authority are preserved. Broader historic architecture-file and status discrepancies are recorded above, not reconstructed as an invented architecture specification.

The first useful slice is implemented but should not be advertised as empirically validated live Claude context until a real session emits the new metadata. Context is a last-reported footprint, not token-by-token streaming measurement or a failure prediction. Account identity remains local scope; Cursor continuous context and Codex attachment semantics need further evidence. Existing primary-screen and UI Automation limitations remain. The native fixture interaction test assumes the pointer is left alone briefly; it restores the user's position afterward.

Recommended next step: run an ordinary Claude Code session and compare its status-line model/context with Verge, including a real model switch and compaction. Only after that evidence, extend the same domain to Codex; do not broaden providers first.


## Codex / ChatGPT extension — 2026-09-08

The same summary, per-session drilldown, pressure policy and attention projection now apply to local Codex sessions under the existing ChatGPT identity. ChatGPT browser/cloud conversations are not claimed as locally observable sessions.

Evidence: OpenAI's [protocol](https://github.com/openai/codex/blob/main/codex-rs/protocol/src/protocol.rs) distinguishes cumulative `total_token_usage` from `last_token_usage`; `TokenUsage::tokens_in_context_window` returns that usage record's `total_tokens`. Its [TUI](https://github.com/openai/codex/blob/main/codex-rs/tui/src/chatwidget.rs) uses `last_token_usage` for context remaining. `turn_context` provides the model and workspace. These schemas were checked against upstream source on this pass.

Implementation uses last-token total / reported model-context capacity, classified Derived. It deliberately reports the raw capacity share, not the TUI's baseline-adjusted remaining percentage. It never substitutes cumulative billing usage or invents a model's capacity. Valid recent readings use the existing 80/90% inspection thresholds. A model change or `compacted` record clears the old context until another token report; invalid/null information is Unavailable; absent information is Unknown. Partial JSON records are skipped without erasing an earlier valid observation in the same tail. Timestamps remain attached to readings, and future/out-of-order records do not overwrite newer information.

No new dependency, daemon, credential access or persistence was introduced. The existing maximum 32 files / 512 KiB per tail is retained. Only model, workspace basename and token metadata enter session presentation. A model or workspace beyond that bounded tail remains unreported. Recent metadata without an activity event is Unknown, not Working or Idle. Token reports extend an explicitly working turn but cannot resurrect a completed turn. The existing recent-rollout discovery is not process-verified live-session discovery. File paths remain internal session keys; human labels use workspace basenames where reported.

Live read-only inspection found three recent Codex sessions, all with context observations, and two with reported models; the remaining model was outside the tail. One explicitly reported model was `gpt-6-astra`. No raw transcripts or identifiers were printed. This confirms positive local ingestion, not complete coverage of every open ChatGPT/Codex conversation.

Verification: `cargo fmt --check`, `cargo test --workspace` (63 tests), production binary build, and the metadata inspection example passed. Added parser coverage for cumulative-vs-context separation, missing/invalid/zero capacity, stale/future data, model changes, compaction and fresh post-compaction readings. The shared UI contract now exercises the ChatGPT identity with identical session details. Native rendering/geometry checks in the workspace suite passed. Live desktop capture returned the desktop background rather than the layered surface, so live visual confirmation of this extension is not claimed.

Codex permission actions remain outside this extension: rollout observation cannot establish a safe request/response channel for approving another client's pending request. Existing Claude approvals and permission precedence are unchanged. Next validation is visual inspection on the interactive desktop and comparison through a real model change/compaction. No Cursor expansion or analytics was added.


### Debug follow-up — 2026-09-08

Resolved two reproduced problems. At 125% scaling, PowerShell screen capture mixed logical and physical coordinates; explicitly setting per-monitor DPI on the capture thread fixes the native navigation test and live capture. The overlay was rendering correctly. Live inspection now confirms the ChatGPT session drilldown with a reported model, context percentage, capacity and activity.

A fixed 512 KiB tail also dropped model/workspace records in long turns (observed latest turn metadata approximately 1.4 MiB behind EOF). A synthetic long-turn regression failed before the fix. The shared rollout reader now starts at 512 KiB and expands only when no valid turn-context record is found, stopping at EOF or 4 MiB. Parsing remains bounded and no transcript content is retained or exposed; metadata beyond the cap stays unknown. This supersedes the extension's fixed-512-KiB and inconclusive-capture limitations above. Re-reading multiple bounded chunks is a deliberate simple implementation; an incremental metadata index is deferred unless measured refresh cost warrants it.

Verified: all 64 workspace tests, formatting, production build, native Enter/Next/Back raster test, and the actual rebuilt ChatGPT drilldown. The previously missing model now resolves to gpt-5.6-sol with a real workspace basename. No new approval behavior or typography changes.
