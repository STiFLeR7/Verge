# Changelog

## 0.1.0 — 2026-09-10

First tagged development release. No prior version exists; this is a
snapshot of the current Windows-complete, Linux-baseline, macOS-experimental
state, not a claim of production readiness.

### Implemented

- Contributor-facing README, architecture, security and troubleshooting guides, plus issue/PR templates.
- Native Windows ambient surface with Inter, agent activity, usage windows and session navigation.
- Claude status-line metadata, local Codex model/context/usage parsing and Windows tool discovery.
- Session-bound Windows Claude approval helper and native request review.
- Shared Rust presentation, a Linux/X11 interactive baseline and an experimental AppKit shell.
- Host-specific portable build scripts and a three-platform CI workflow.
- Native Windows and Linux E2E coverage for navigation and inactivity; Windows permission UI and transport contracts.
- Live real-time verification of the portable Windows binary and an adversarial edge-case audit of the local-data parsing/domain-math layer (malformed JSON, clock skew, permission caps, unicode); 3 new regression tests, 71 workspace tests total. See [edge-case report](docs/design/E2E_EDGE_CASES_2026-09-10.md).

### Known limits

- macOS Swift compilation and native execution remain unverified.
- Linux visual/accessibility parity and native Wayland are unfinished.
- Direct approval is available only for Claude on Windows; its installer requires a supported existing status-line setup.
- Public licensing, signed distribution and automatic updates have not been established.

See [platform status](docs/design/PORTABILITY.md) and the [verification report](docs/design/E2E_PORTABILITY_2026-09-10.md). This is a pre-release development tag, not a production release guarantee.
