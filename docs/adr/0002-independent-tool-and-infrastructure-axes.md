# 0002 — Independent tool and infrastructure capability axes

## Context

`codenotch`'s adapters fuse "how do I get bytes/credentials" with "how do I
interpret them" into one per-vendor module. That couples an unrelated pair
of concerns: a Cursor release breaking parsing has nothing to do with a
Windows Credential Manager API changing.

## Decision

Two independent axes, neither collapsed into the other:

- **Axis A — Tool Adapters** (`tools/*`): own interpretation of one vendor's
  local state/schema. Never call a raw OS API directly.
- **Axis B — Infrastructure Capabilities** (`platform/*`, one contract per
  capability, declared in `core/ports`): `CredentialStore`, `OverlaySurface`,
  and others, each with its own interface, composed by a tool adapter, never
  reimplemented by one.

## Reasoning

Validated directly, not just designed on paper, by this repository's first
vertical slice: `verge-tool-claude::ClaudeCodeUsageSource` is generic over
`C: CredentialStore` and never touches a Win32 API; `verge-platform-windows`
never knows what Claude Code's `stats-cache.json` looks like. The seam holds
under a real implementation, not only in the abstract.

## Consequences

Adding a second tool (Cursor) will reuse `WindowsCredentialStore` unchanged
if Cursor's Windows credential also turns out to be a local file at a known
path — the adapter only needs a new lookup key, not new platform code. If
Cursor's Windows credential is *not* a local file, `WindowsCredentialStore`
gains a second, real Credential-Manager-backed path behind the same trait,
and no tool adapter changes.

## Rejected alternatives

One per-OS "shell adapter" bundling credential access, file watching, window
control, tray, login, and updates together — rejected per
`docs/PRODUCT_ARCHITECTURE.md` §4: it forces unrelated changes (a tray-icon
fix, a SQLite-read fix) through the same module for no reason but "same OS."
