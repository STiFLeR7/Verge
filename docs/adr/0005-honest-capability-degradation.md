# 0005 — Honest capability degradation

## Context

`codenotch`'s `ProviderStatus`/`Fidelity` design never invents a number, and
that discipline is the single most important thing worth carrying forward
(`docs/PRODUCT_ARCHITECTURE.md` §2). A cross-platform product adds a second
place invention can creep in: pretending a capability (the ambient overlay
itself) exists when the OS/desktop cannot actually provide it.

## Decision

Two structural rules, enforced in code, not just convention:

1. **Data:** `Availability` must be one of its explicit variants
   (`Available`, `Unauthenticated`, `AccessDenied`, `NotMetered`,
   `Unsupported`, `Unreachable`, `Error`) before a `UsageWindow` is ever
   constructed. `project_ambient_state` (`core/src/domain/ambient_state.rs`)
   enforces this structurally: it returns `None` for
   `most_constrained_window` whenever `availability != Available`,
   regardless of what a buggy caller might have put in `windows`.
2. **Capability:** `ProductMode` is selected only from a `CapabilityProfile`
   the platform layer actually reported (`core/src/application/product_mode.rs`).
   A capability nothing reported is `None`, not `Full` — there is no default
   branch that assumes success.

## Reasoning

Both rules are exercised by tests today, not just asserted in a doc:
`unavailable_snapshot_never_yields_a_window`,
`no_windows_yields_none_even_when_available`, and
`missing_overlay_capability_yields_on_demand_not_full` all fail if this
invariant is ever silently broken by a future change.

`tools/claude`'s adapter follows the same discipline end-to-end: a tool with
zero recorded activity returns `Availability::NotMetered`, never a fabricated
`Count(0)` (`credential_present_but_no_activity_recorded_is_not_metered`).

## Consequences

Every future tool adapter and every future platform implementation must
route through these same two chokepoints rather than inventing a
`ProductMode` or a `UsageSnapshot` ad hoc — this is a deliberate constraint
on future code, not a one-time decision.

## Rejected alternatives

A single flat status enum, or scattering `if cfg!(windows)`-style
mode-selection logic through UI code — both rejected per
`docs/PRODUCT_ARCHITECTURE.md` §6 and §15.
