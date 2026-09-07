# Domain tests

Live inline as `#[cfg(test)]` modules next to the code they test, per normal
Rust convention — not duplicated here:

- `core/src/domain/ambient_state.rs` — the "never invent usage" projection
  invariant.
- `core/src/domain/recency.rs` — `Live`/`Recent`/`Aged` classification.
- `core/src/domain/capability_profile.rs` — unreported capability defaults.
- `core/src/application/product_mode.rs` — `CapabilityProfile` → `ProductMode`.

Run with `cargo test -p verge-core`. Pure, deterministic, no OS, no network —
see `docs/PRODUCT_ARCHITECTURE.md` §19.
