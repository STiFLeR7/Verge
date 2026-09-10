# Changelog

## Unreleased — 0.1.0 development

### Implemented

- Contributor-facing README, architecture, security and troubleshooting guides, plus issue/PR templates.
- Native Windows ambient surface with Inter, agent activity, usage windows and session navigation.
- Claude status-line metadata, local Codex model/context/usage parsing and Windows tool discovery.
- Session-bound Windows Claude approval helper and native request review.
- Shared Rust presentation, a Linux/X11 interactive baseline and an experimental AppKit shell.
- Host-specific portable build scripts and a three-platform CI workflow.
- Native Windows and Linux E2E coverage for navigation and inactivity; Windows permission UI and transport contracts.

### Known limits

- macOS Swift compilation and native execution remain unverified.
- Linux visual/accessibility parity and native Wayland are unfinished.
- Direct approval is available only for Claude on Windows; its installer requires a supported existing status-line setup.
- Public licensing, signed distribution and automatic updates have not been established.

See [platform status](docs/design/PORTABILITY.md) and the [verification report](docs/design/E2E_PORTABILITY_2026-09-10.md). No released version is implied by this development log.
