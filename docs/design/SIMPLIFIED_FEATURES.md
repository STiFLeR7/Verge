# Simplified agent rail — 2026-09-08

> Historical snapshot, superseded by `../../PRODUCT.md`, `../../DESIGN.md`, `SESSION_INTELLIGENCE.md`, and `CLAUDE_APPROVAL_SETUP.md`. The approval, tint, geometry, provider coverage and test-count statements below describe that earlier metadata-only stage, not current behavior. Current Verge can explicitly approve or deny Claude requests when its synchronous hook is installed.

The rail keeps Codenotch's 44/117 frame scale: 69.9487 DIP width, 44 DIP usage track and 17.2991 DIP identity. One agent occupies one cell, with a small session count above the ring and the most constrained usage percentage below it. Hover reveals current-session and weekly usage bars, reset times and brief activity context. [Native synthetic preview](evidence/simplified-usage-expanded.png).

## Working features

- **Sessions:** PID and creation-time-verified Claude sessions; counts above 99 shorten to `99+` in the rail, with the exact count in the heading.
- **Usage and limits:** the existing Claude status line exports only documented `rate_limits.five_hour` and `rate_limits.seven_day` percentages and reset timestamps. All windows survive core projection. Aged data dims; expired windows disappear until another reading arrives. Context-token percentages are never mistaken for subscription usage.
- **Reminders:** a 12-second reveal at 80%, 95% and 100%, once per threshold/reset window during the current Verge run. The reveal never requests keyboard focus and remains open if hovered. Restarting Verge resets reminder deduplication. No background notification service or scheduled Codex task is involved.
- **Approval indication:** permission-request and permission-prompt notification hooks emit yellow attention state and a short tool label. Decisions remain in Claude; Verge neither approves nor rejects requests. Command contents and prompts are not saved.
- **Tints:** green for prompt/tool work; yellow for approval waiting; red after a Stop/completion event. Unknown/idle without an explicit completion signal remains neutral. These are common presentation states; only Claude is currently wired on Windows.

The existing terminal status-line output is preserved. `scripts/install-claude-signals.ps1` installed metadata-only hooks and a 15-second status-line refresh when no interval existed. It saved originals as `~/.claude/settings.json.verge-backup` and `~/.claude/statusline-command.js.verge-backup`. Metadata lives in `%LOCALAPPDATA%/Verge/signals`; no credential access or extra usage API request is needed. Hook changes may require a new Claude session to take effect.

## Evidence

The live read-only probe observed **two verified sessions**, **32% current-session usage**, **65% weekly usage**, and both real reset timestamps. These are observations at verification time, not hardcoded values.

`cargo test --workspace` passes 48 tests. `node scripts/claude-signal.test.cjs` checks metadata filtering, invalid input, event mapping, installer idempotency, backups, and unchanged status-line output. A native reminder fixture automatically expanded to 369 pixels at 125% DPI without becoming the foreground window. The full hover test was interrupted by user pointer movement; the previous rail geometry/hover validation remains recorded separately. Actual working/approval/completion events were not artificially generated in the live session; those mappings are verified with isolated test data.

Sources: Claude's [status-line field reference](https://code.claude.com/docs/en/statusline) and [event hooks](https://code.claude.com/docs/en/hooks). These supply usage/reset fields and lifecycle events; this integration does not use hook decision output.
