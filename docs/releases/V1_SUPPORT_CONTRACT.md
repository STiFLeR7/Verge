# Verge v1.0.0 support contract

This document defines the support claims Verge must satisfy before the
`v1.0.0` tag is published. Until every release gate is closed, the current
build remains pre-release software and the narrower claims in the README
apply.

## Supported platform tiers

| Platform | v1.0.0 tier | Required behavior |
|---|---|---|
| Windows 11 x64 | Supported reference implementation | Native Win32 surface, Inter, provider discovery, usage and session detail, 30-second inactivity collapse, display/DPI recovery, and optional request-bound Claude approval. |
| Ubuntu 24.04 x86_64 under X11 or XWayland | Supported X11 implementation | Native X11 surface, Inter, local provider/session presentation, theme-aware idle state, session navigation, inactivity collapse, and display/work-area recovery. Direct approval is unavailable. |
| macOS 13+ on Apple silicon | Supported AppKit implementation | Swift/AppKit nonactivating panel, bundled Inter, local provider/session presentation through `verge-state`, session navigation, inactivity collapse, and display recovery. Direct approval is unavailable. |

Native Wayland and Intel macOS are unsupported in v1.0.0. Other Windows,
Linux distribution, architecture, compositor, and macOS combinations are
unverified unless a later support contract names them.

## Integration boundary

Verge reads bounded local metadata and presents only values that the source
reported or that Verge can derive with an explicit fidelity. Installation or
process discovery alone does not prove an active session, quota, or approval
capability.

Direct approval remains limited to Claude on Windows. The optional helper
must preserve session/process identity, one-shot decisions, timeouts, and
fail-closed behavior. Codex, other providers, Linux, and macOS have no direct
approval channel in this release.

## Distribution

The release formats are:

- Windows: signed x64 ZIP containing `verge.exe`, `verge-state.exe`, and the
  optional `verge-claude-hook.exe`.
- Linux: x86_64 tarball built on Ubuntu 24.04 and requiring X11 or XWayland.
- macOS: signed and notarized Apple-silicon application ZIP for macOS 13+.

Each release includes the governing distribution terms, third-party notices,
SHA-256 checksums, manual update instructions, and complete removal steps.
There is no background updater in v1.0.0.

## Release gates

The tag is blocked until all of the following are recorded in
`docs/releases/V1_VERIFICATION.md` for the exact release revision:

- the repository owner has selected public licensing or proprietary
  distribution terms;
- native Windows, Linux, and macOS capability contracts pass;
- multi-monitor/display-change behavior is verified for each supported tier;
- a live Claude host reload, approval, denial, offline failure, and surgical
  uninstall/recovery pass on Windows;
- an eight-hour Windows soak and two-hour Linux and macOS soaks complete
  without a crash or unbounded resource growth;
- Windows and macOS signatures, macOS notarization, package contents, and
  checksums verify;
- README, PRODUCT, DESIGN, SECURITY, PORTABILITY, CHANGELOG, and release copy
  agree with this contract.

## Excluded from v1.0.0

The release does not add a settings/dashboard surface, usage history,
analytics, costs, forecasts, exports, sound or OS notifications, terminal
focus tracking, inferred stalled state, session-management actions, new
providers, new direct-approval providers, or native Wayland.
