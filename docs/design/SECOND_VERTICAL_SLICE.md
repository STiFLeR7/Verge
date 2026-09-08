# Second Vertical Slice — Codex + Linux/X11

An architectural validation exercise, not a feature-breadth exercise. The
first slice (`docs/design/claude-code-windows-local-state.md`) proved one
point on each axis: Claude Code (tool) + Windows (platform). This slice adds
the *second* point on both axes **simultaneously** — Codex (tool) +
Linux/X11 (platform) — to test whether the two-axis split
(`docs/PRODUCT_ARCHITECTURE.md` §4) actually generalizes, or only happened
to work for the one pair it was designed against.

## Scope

Intentionally tested: a Codex `UsageSource` reading real, empirically
discovered Linux local state; a Linux/X11 `CredentialStore` and
`OverlaySurface`; a live-running ambient overlay rendering that data on a
real (nested) X server; contract tests against realistic fixtures; one new
cross-tool domain test. Intentionally **not** touched: Cursor, any other
tool, Wayland, `ui/detail`, Tauri, packaging, telemetry, auto-update,
authentication infrastructure, agent control — all explicitly out of scope
per the task brief.

## Baseline (before this slice)

5 crates (`verge-core`, `verge-tool-claude`, `verge-platform-windows`,
`verge-ui-ambient-windows`, `verge-desktop`), 23 passing tests, git clean at
commit `544acf5`. Verified directly (`cargo test --workspace`) before any
change was made.

## Codex findings

Full detail, with per-claim confidence labels, in
`docs/design/codex-linux-local-state.md`. Summary:

- **Install**: official `codex-x86_64-unknown-linux-musl.tar.gz` release
  binary (Apache-2.0, `github.com/openai/codex`, tag `rust-v0.153.4`) — a
  static Rust binary, no Node.js involved on this path.
- **Auth — CONFIRMED (source)**: `$CODEX_HOME/auth.json` (`$CODEX_HOME` env
  override, else `$HOME/.codex`), a plaintext JSON file
  (`AuthCredentialsStoreMode::File` is the `#[default]`), same posture as
  Claude Code's Windows credential. Configurable to `Keyring`/`Auto`/
  `Ephemeral` — documented as a named, accepted limitation, not silently
  assumed away.
- **Usage — CONFIRMED (source), and genuinely different from Claude
  Code**: no local usage/quota cache exists anywhere. Rate limits are parsed
  only from live HTTP response headers on an authenticated call. The
  adapter reports `Availability::Unsupported` rather than inventing a
  number from an unrelated local signal.
- **Activity — PARTIALLY CONFIRMED**: a real signal exists
  (`thread_history_1.sqlite`'s `thread_turns` table), not implemented this
  pass — Claude Code has no `ActivitySource` either, kept symmetric on
  purpose rather than asymmetric for an unrelated reason.
- **Account discovery — CONFIRMED**: one account per `$CODEX_HOME`;
  `--profile` is config, not identity.
- One item left **UNKNOWN** on principle: the exact on-disk shape of a real,
  logged-in `auth.json` — no real login was performed, per the task's
  explicit instruction.

## Linux/X11 findings

Live-verified in a real Xephyr + metacity nested X session (WSLg's own
RAIL-remoted X11 was tried first and abandoned — see Architectural friction
below):

| Capability | Result | Evidence |
|---|---|---|
| Borderless | Confirmed | `xephyr_text_rendering_fixed.png` |
| Transparent | Confirmed | same |
| Correct placement | Confirmed | same |
| Always-on-top over a normal window | Confirmed, after a real fix (below) | `xephyr_clickthrough_typed_text.png` |
| Click-through | Confirmed, both directions (pointer routing *and* keyboard focus transfer) | `xephyr_clickthrough_typed_text.png` — `CLICKTHROUGH-OK` typed and landed in the terminal underneath, overlay text still rendered on top |
| Interactive region | N/A this slice (no interactive control — same deliberate scope as Windows) | — |
| Survives target window close | Confirmed | overlay unaffected after `pkill xterm` |
| Clean shutdown | Confirmed | `SIGTERM` → clean exit |
| State reaching the overlay | Confirmed | overlay rendered "Verge · Codex / Needs sign-in" — real output of `CodexUsageSource` → `LinuxCredentialStore`, no real login present in this environment |
| DPI / multi-monitor / session lifecycle | **NOT TESTED** | no hardware/scope for this pass |
| True bare-metal X11 | **NOT TESTED** | only nested WSLg/Xephyr available |

Two real, genuine bugs were found and fixed during this slice's own
bring-up (not carried over from the spike), documented inline in
`platform/linux/x11/src/overlay.rs`:

1. `_NET_WM_STATE_ABOVE` has no effect on an override-redirect window (no WM
   manages it at all), and X11's default stacking puts whatever is mapped
   *most recently* on top. Fixed with a periodic `ConfigureWindow`/
   `StackMode::ABOVE` on the same cadence as content redraw — the X11
   analogue of Windows' periodic `HWND_TOPMOST` re-assertion
   (`platform/windows/src/overlay.rs`'s own documented finding from the
   first slice).
2. `PolyText8`'s `items` field is raw wire bytes (a length-prefixed
   `TEXTITEM8`), not plain text — passing UTF-8 directly produced a silent
   `BadLength` X error. Fixed with an explicit encoder
   (`text_item8()`) that also degrades non-Latin-1 characters to `?` rather
   than corrupting the item length.

**Not over-promised:** fullscreen-specific behavior was not part of this
slice's evidence at all (the Windows slice's own fullscreen test was
already only `PARTIAL`, per `SPIKE_RESULTS.md`); this slice's environment
(nested WSLg/Xephyr) cannot exercise bare-metal GPU compositing, so no claim
is made about it here one way or the other.

## Runtime demonstration

`verge-linux-x11` (a second `[[bin]]` in the `apps/desktop` package) ran
inside the WSL `Ubuntu` distro against a nested Xephyr + metacity X server,
wiring `CodexUsageSource` (reading a real, unauthenticated
`LinuxCredentialStore` lookup) → `project_ambient_state` →
`X11OverlaySurface`, rendering "Verge · Codex / Needs sign-in" live,
on-screen, at the correct screen-edge position, throughout a target
`xterm`/`cat` window opening, being typed into (click-through proof), and
closing.

## Tests

**33 passing, 0 failed** (`cargo test --workspace`, re-run and independently
verified from the Windows side after the fork's report, not just trusted):
`verge-core` 13 (+1 new: `unsupported_availability_never_yields_a_window_for_a_second_tool`),
`verge-platform-windows` 2, `verge-platform-linux-x11` 2 (new),
`verge-tool-claude` 3 + 4 contract, `verge-tool-codex` 0 + 5 contract (new),
`verge-ui-ambient-windows` 2, `verge-ui-ambient-linux-x11` 2 (new). Up from
23 at baseline. `cargo fmt --check` was clean for the new code but flagged
drift across the *entire* pre-existing codebase (rustfmt had never been run
since the first commit) — `cargo fmt` was applied workspace-wide as a
mechanical, zero-logic-change fix, and the full suite was re-run and
confirmed green afterward.

## Architectural validation

Answering the Phase 12 audit questions directly, based on reading every new
and modified file, not just the self-report:

- **Tool → OS leakage: none found.** `tools/codex/src/lib.rs` contains zero
  X11/Linux-specific code — no `/proc`, no raw fds, no path assumptions
  beyond what `CredentialStore::find(key)` hands back as an opaque outcome.
- **OS → Tool leakage: none found.** `platform/linux/x11/src/overlay.rs`
  and `credential_store.rs` do not reference Codex by name anywhere except
  in doc comments explaining *why* a design choice was made — the code
  itself only knows about a `key: &str` and a `CredentialOutcome`.
- **Domain contamination: none found.** `core/domain` and `core/ports`
  gained exactly one new test in `ambient_state.rs` and zero new types.
  `Availability::Unsupported`, introduced in the first slice for a
  platform-capability gap, **generalized cleanly to a tool-inherent gap**
  (Codex has no local usage signal at all) without needing a new variant —
  a genuine positive signal for the enum's original design, confirmed by a
  second, independent use case rather than assumed.
- **Port quality: unchanged and sufficient.** `UsageSource`,
  `CredentialStore`, `OverlaySurface` all served this slice exactly as
  declared in the first slice. No port's shape needed to change; none was
  forced open to accommodate Codex or X11.
- **Application layer: still generic.** `select_product_mode` and
  `project_ambient_state` were called unmodified from `main_linux_x11.rs`
  with no new branch, flag, or special case for "Codex" or "Linux."
- **Duplication — one legitimate item found, deliberately not resolved
  yet**: `render()` in `ui/ambient/windows` and `ui/ambient/linux-x11` is
  verbatim-identical formatting logic. Left duplicated on purpose: two data
  points are not enough to know whether a third OS's ambient surface would
  actually want the same rendering, and the task's own instruction was
  explicit not to abstract on resemblance alone. This is flagged here so it
  isn't forgotten, not silently accepted as permanent.

**Verdict on the central question — no tool adapter knows the operating
system, and no OS implementation knows which tool it's serving — holds,
verified by reading the code, not by taking the claim at face value.**

## Architectural friction

Two real points of friction surfaced, both resolved without changing any
`core/ports` contract:

1. **Two binaries sharing one `apps/desktop` package's `[dependencies]`
   table broke cross-compilation.** The moment `verge-ui-ambient-linux-x11`
   became a real dependency of the same package as
   `verge-ui-ambient-windows`, an unconditional `use` of a
   `cfg(windows)`-gated item on one side (and the symmetric case on the
   other) failed to compile for the *other* target. Fixed by cfg-gating
   each `run_ambient_shell` function per OS while keeping each crate's pure
   `render()` formatting function ungated (so it stays unit-testable on
   every platform), and giving each `main*.rs` a stub `fn main()` behind
   `#[cfg(not(<os>))]` for the platform it doesn't target. This is now a
   named, reusable pattern — see ADR 0006 — for when a third OS is added.
2. **WSLg's per-window (RAIL-style) remoting made host-desktop screenshot
   verification actively misleading**, not just inconvenient: each X11
   window becomes an independent Windows HWND, and observed z-order didn't
   track X11-internal stacking calls at all. Switching to Xephyr (a real,
   self-contained nested X server) and reading state via `xdotool`/direct
   X11-internal screenshots was the only reliable verification path. Not an
   architecture problem, but a real cost — documented here so a future
   Linux/Wayland pass doesn't re-spend the time discovering it.

No instance was found where a tool adapter needed OS knowledge, an OS
implementation needed to know which tool it served, or a generic
`LinuxAdapter`-style catch-all became tempting. Nothing here required
stopping to report an unresolvable conflict per the task's Phase 8
instruction — friction was real but resolvable inside the existing
boundaries.

## Changes to architecture

None to `core/ports`' contracts. The one durable, generalizable decision
this slice produced — the per-OS binary/module cfg-gating pattern for a
shared `apps/desktop` package — is recorded as
`docs/adr/0006-per-os-binary-composition.md`.

## Remaining unknowns

- **Linux Wayland, KDE, GNOME**: untouched by this slice (out of scope, per
  the task brief) — see `D:\overlay-capability-spike\SPIKE_RESULTS.md` for
  the separate, earlier, partial evidence on these.
- **Multi-monitor, DPI-across-displays, session lifecycle (X11)**: not
  tested — no hardware, and deliberately not simulated to avoid a false
  confidence claim.
- **Real Linux credential-store (Secret Service/GNOME Keyring) behavior**:
  not tested. Codex's own `Keyring`/`Auto` store modes exist but weren't
  exercised — `LinuxCredentialStore` only covers the default `File` mode,
  documented as a named limitation, not a silent gap.
- **Codex's live usage endpoint**: still unavailable to this adapter by
  design (no network call is made) — this is not a gap to close later so
  much as a confirmed, permanent shape for a local-state-only adapter; an
  `Official`-fidelity Codex usage signal would need a genuinely different
  `UsageSource` implementation that performs a real authenticated network
  call, a separate, larger decision not made here.
- **Codex activity detection**: a real signal exists
  (`thread_history_1.sqlite`) but is not implemented — same open status as
  Claude Code's own activity detection.
- **Exact on-disk shape of a real, authenticated `auth.json`**: UNKNOWN by
  design — no real login was performed during this investigation.

## Verdict

```text
ARCHITECTURE HOLDS
```

Not because the code compiles — because the two-axis split was checked by
reading every new file for tool-knows-OS or OS-knows-tool leakage and found
none, because the "never invent usage" invariant was independently
re-exercised by a second tool with a *structurally different* failure mode
(no local signal at all, not just "not yet authenticated") and held without
a new domain type, and because the one real friction point found (binary
cross-compilation) was resolved inside existing boundaries rather than by
weakening a contract.
