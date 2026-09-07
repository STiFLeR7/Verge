# Contract tests

Live per-crate, one fixture file per (tool, local-state shape) pair, per
normal Rust convention — not duplicated here:

- `tools/claude/tests/contract_local_state.rs` — pins how
  `ClaudeCodeUsageSource` interprets a `stats-cache.json`-shaped fixture,
  against a fake `CredentialStore`. No real credential is ever used.

Run with `cargo test -p verge-tool-claude`. See
`docs/PRODUCT_ARCHITECTURE.md` §19 and
`docs/design/claude-code-windows-local-state.md` for how the fixture shape
was established.
