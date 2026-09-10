# Verge E2E verification — 2026-09-10

## Results

| Check | Result | Evidence and scope |
|---|---|---|
| Windows workspace | PASS, 68 tests | `cargo test --workspace --quiet` |
| Linux workspace | PASS, 56 tests | Native Ubuntu 24.04 WSL, `cargo test --workspace --quiet` |
| Rust formatting | PASS | `cargo fmt --all --check` |
| Claude metadata/installer | PASS | `node scripts/claude-signal.test.cjs` |
| Pi/Kilo integrations | PASS | `node scripts/integrations/test.mjs` |
| Claude and ChatGPT Windows sessions | PASS | Open details, mouse previous/next, noninteractive count, wrap both ways, Back, Escape. Native posted messages; synthetic sessions. |
| Windows permission UI | PASS | Deny, Dismiss, Approve through full review, closing review denies. Synthetic request only. |
| Windows helper/broker | PASS | Real pipe: approve, deny, dismiss, disconnect, terminated parent; wrong identity and repeated decisions rejected. Isolated synthetic ancestor, no tool executed. |
| Windows window contract | PASS | 125% DPI, 1920-pixel right edge, 95-pixel compact width, six physical hover cycles with 1800ms source delays, dynamic input behavior, focus retained; GDI 1 to 1. |
| Windows working inactivity | PASS | Separate targeted run: actual 30-second timer collapses to 6 pixels; physical hover wakes it. |
| Linux native session navigation | PASS | Packaged release executable under authenticated Xvfb. Real mouse clicks plus XGetImage pixel comparisons verify details, next, count no-op, previous, wrap, and Back. |
| Linux working inactivity | PASS | Two temporary working Codex rollout fixtures; actual timer collapses to 6-pixel height, hover reveals, focus retained. |
| macOS Rust cross-check | PASS | `cargo check --workspace --all-targets --target aarch64-apple-darwin`; this does not compile Swift/AppKit. |

## Reproductions and changes

No product-code failure was reproduced in these checks. Two attempts to extend the combined Windows hover/inactivity run stopped because the real pointer moved away from its asserted position (expected 1872,475; observed 1919,202 and 1779,110). The six-cycle hover test had already passed. Added `-InactivityOnly` to run the new timer check independently; it passed without removing any hover assertions from the normal run.

Extended `platform/linux/x11/tests/native_smoke.py` with native X11 pixel comparisons for session navigation. Added the actual timer/reveal assertions to `platform/windows/tests/window_contract.ps1`. No application behavior or permission policy was changed in this E2E pass.

## Repeat commands

Build the Windows fixtures with `cargo build --release -p verge-platform-windows --example visual_states` and `cargo build -p verge-desktop --example permission_contract --bin verge-claude-hook`. Run each Windows PowerShell contract in a separate `pwsh -NoProfile -File` process; the session contract accepts `-Brand Claude` and `-Brand ChatGPT`. The window contract optionally accepts `-InactivityOnly`. Close the ordinary Verge overlay for those native fixture tests and restore it afterward; do not run multiple UI fixtures concurrently.

On ordinary Linux: `xvfb-run -a python3 platform/linux/x11/tests/native_smoke.py dist/linux/verge`. On this WSL host: `VERGE_TEST_TCP=1 xvfb-run -a -l --server-args="-screen 0 1280x1024x24 -listen tcp" python3 platform/linux/x11/tests/native_smoke.py dist/linux/verge`. Xvfb keeps its temporary authentication. The test isolates HOME, CODEX_HOME, USERPROFILE and LOCALAPPDATA and deletes only its temporary fixture directory.

## Evidence and limits

Windows run logs: `target/e2e-2026-09-10/`. Refreshed session screenshots: [ChatGPT details](evidence/e2e-chatgpt/first.png), [ChatGPT Back](evidence/e2e-chatgpt/back.png), [Claude overview](evidence/e2e-claude/back.png). Representative screenshots were visually inspected for Inter text, session context and separate navigation controls.

The Windows portable app was restored after testing. Its configured Claude gate points to `D:/Verge/dist/windows/verge-claude-hook.exe` with timeout 130 seconds. Configuration inspection and isolated transport tests do not prove a running Claude host has reloaded that hook. No real Claude tool approval or denial was submitted.

macOS Swift compilation and native UI execution remain unverified. Native Wayland, Linux final typography/theme/monitor behavior, screen-reader accessibility, multi-monitor scenarios, provider-host reloads and long-duration soak testing are outside this result. The Linux surface remains a functional X11 baseline, not visual parity with Windows. This report is not an all-platform release certification.
