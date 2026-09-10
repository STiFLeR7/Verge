# Verge edge-case and real-time verification — 2026-09-10

This pass complements [E2E_PORTABILITY_2026-09-10.md](E2E_PORTABILITY_2026-09-10.md)
(window/UI/navigation/permission contracts). It targets two things that
report did not cover: a live real-time run of the actual portable binary,
and an adversarial audit of the pure-Rust parsing/math layer (`tools/claude`,
`tools/codex`, `core/src/domain`, `core/src/application`) against malformed
or boundary local data.

## Live real-time run

`dist/windows/verge.exe --open` launched against this machine's real Claude
installation (product mode `FULL`). Observed over 40+ seconds, spanning the
30-second inactivity-collapse boundary: process stayed alive, memory flat
(~14 MB, no growth), no crash, clean `taskkill` shutdown. Not a soak test —
a real-time sanity check that the current build behaves under an actual
live data source, not just fixtures.

## Edge-case audit of local-data parsing and domain math

Full read of `tools/claude/src/*.rs`, `tools/codex/src/*.rs`,
`core/src/domain/*.rs`, `core/src/application/*.rs` against the project's
own invariant — missing or bad data must degrade to an explicit
`Unavailable`/`Unsupported` state, never a fabricated value or a panic.

| Category | Result |
|---|---|
| Malformed/truncated JSON (stats-cache, Codex rollout, status-line) | Already safe — every parse path is `Option`/`Result`-chained to explicit unavailable states; no `.unwrap()` on untrusted data |
| Missing fields / null / wrong JSON type | Already safe — `#[serde(default)]` plus checked accessors short-circuit |
| Zero-limit division, negative percentage, >100%, NaN | Already safe — explicit `(0.0..=100.0)` range and `is_finite()` guards in both usage-limit parsers |
| Reset time in the past/future, clock skew | Guard existed but was untested — added `clock_skew_future_as_of_is_live_not_negative_age` (`core/src/domain/recency.rs`) |
| Context readings timestamped after `now` | Guard existed but was untested — added a case to `ContextUsage::pressure()`'s existing test (`core/src/domain/session.rs`) |
| Unicode / very long strings in names and paths | Already safe — control-character and bidi-override stripping plus a 160-character cap in both adapters |
| Duplicate/colliding session IDs, PID reuse | Already safe — covered by existing `activity.rs` tests |
| `PermissionBook` in-flight cap (8) and request-ID overflow | Guards existed but were untested — added `register_rejects_beyond_capacity_and_on_id_overflow` (`core/src/domain/permission.rs`) |
| `.unwrap()`/integer overflow sweep across all six source files | No panic risk found beyond the three now-covered branches above |

No production logic changed. Three regression tests added for previously
correct-but-unexercised defensive branches. Workspace tests: 68 → 71,
all passing; `cargo fmt --all --check` clean.

## What this does not cover

Multi-monitor placement, Linux/macOS live runs, Wayland, screen-reader
accessibility, long-duration (multi-hour) soak testing, and a live Claude
host reloading the approval hook are unverified here, same as the prior
report. This is a Windows-only, single-session pass.
