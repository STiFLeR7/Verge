# Verge

Verge is an ambient instrument for developers using AI coding tools. It answers what is happening with minimal interruption, attached to the right display edge.

## Platform

Windows remains the most complete implementation, using Rust and Win32 layered rendering. Linux/X11 and Swift/AppKit macOS surfaces are in progress; consult the [platform matrix](docs/design/PORTABILITY.md) and [v1 support contract](docs/releases/V1_SUPPORT_CONTRACT.md). Preserve borderless transparency, always-on-top, per-monitor DPI awareness, and dynamic WS_EX_TRANSPARENT interaction. D:\codenotch is a design reference, not a source-code dependency.

## Confirmed scope

The user approved the attached redesign brief on 2026-09-08. Keep real application state and the Core → AmbientState → presentation → native surface boundary. Claude and local Codex session/usage sources are implemented on Windows; other tools have varying discovery and observer support. No settings surface, history, analytics, or cloud features.

## Evidence and limits

Current session-awareness work is described in
[SESSION_INTELLIGENCE.md](docs/design/SESSION_INTELLIGENCE.md). Earlier
Claude-only and no-direct-approval descriptions below are historical: Windows
now reads Codex usage/activity and supports request-bound Claude approvals.
The first session/model/context slice targets Claude, with native tooltip
drilldown and no additional ambient dashboard.

Windows composes verified Claude sessions with official status-line usage/reset values. It shows both quota windows and threshold reminders, and uses metadata hooks for working, approval waiting and completion. When the optional synchronous hook is installed, Verge can approve or deny a request after native review; the broker boundary is documented in [SECURITY.md](SECURITY.md). Other observers remain metadata-only, and provider coverage varies as recorded in [PROVIDER_DISCOVERY.md](docs/design/PROVIDER_DISCOVERY.md).

## Brand commitment

Dense black material, authentic tool identity, localized green/yellow/red activity light, curved right-edge attachment, restrained native type, and continuous compact-to-expanded motion. Respect reduced motion. No provider directory or dashboard.
