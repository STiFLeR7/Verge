# Contributing to Verge

Verge is in development. Start with the [README](README.md) and [platform matrix](docs/design/PORTABILITY.md). Repository access does not override the current [license](LICENSE); external contribution terms have not been established.

## Set up

Install current stable Rust with rustfmt. Use MSVC C++ build tools and the Windows SDK on Windows, a C linker on Linux, or Xcode command-line tools and Swift on macOS. Install Node.js for observer development/tests and PowerShell 7 (`pwsh`) for the Windows native test commands. Native Linux tests additionally need Python 3, Xvfb, xauth, xdotool and the X11 runtime library.

From the repository root:

```sh
cargo build -p verge-desktop --bin verge
cargo test --workspace
```

For Windows/Linux development, run `cargo run -p verge-desktop --bin verge`. For macOS, use `bash scripts/build-portable.sh` and open `dist/Verge.app`; the Rust launcher alone does not build the Swift shell. See the README for portable builds.

## Where changes belong

| Area | Location |
|---|---|
| Usage, sessions, freshness, permissions and capability types | `core/` |
| Tool-owned schemas and metadata parsing | `tools/claude/`, `tools/codex/` |
| Windows rendering, process checks, discovery and permission transport | `platform/windows/` |
| X11 window and input behavior | `platform/linux/x11/` |
| Shared presentation mapping | `ui/ambient/shared/` |
| Native UI composition | `ui/ambient/windows/`, `ui/ambient/linux-x11/`, `ui/ambient/macos/` |
| Application wiring and local presentation bridge | `apps/desktop/` |
| Observer integrations and packaging | `scripts/` |

Use the existing boundaries. Keep provider parsing out of window code and OS APIs out of the domain. A discovered executable is not evidence of work or available quota. A passive observer must never grant permission.

## Before requesting review

```sh
cargo fmt --all --check
cargo test --workspace
cargo check --workspace --all-targets
node scripts/integrations/test.mjs
```

On Windows:

```powershell
node scripts/claude-signal.test.cjs
```

Run the native checks relevant to your change. Cross-compiling Rust does not validate native rendering or compile the Swift shell. The [portable CI workflow](.github/workflows/portable.yml) defines native host builds; distinguish locally verified results from CI jobs that have not run.

Keep changes focused. Add a regression test for a reproduced bug, test unknown/stale/malformed data at parser boundaries, and explain why a layout or security change is necessary. Follow the existing Inter typography and [design vocabulary](DESIGN.md). Do not change frozen styling as part of an unrelated fix.

## Native Windows checks

Build fixtures:

```powershell
cargo build --release -p verge-platform-windows --example visual_states
cargo build -p verge-desktop --example permission_contract --bin verge-claude-hook
```

Close the normal Verge process while running these fixtures, then restore it. The UI tests move the pointer and should run sequentially on an unlocked desktop. If Claude's gate is installed, requests will be denied while its normal broker is unavailable.

```powershell
pwsh -NoProfile -File platform/windows/tests/session_ui_contract.ps1 -Brand Claude
pwsh -NoProfile -File platform/windows/tests/session_ui_contract.ps1 -Brand ChatGPT
pwsh -NoProfile -File platform/windows/tests/permission_ui_contract.ps1
pwsh -NoProfile -File platform/windows/tests/window_contract.ps1
./target/debug/examples/permission_contract.exe
```

The window test takes over 30 seconds to exercise inactivity. `-InactivityOnly` skips the repeated hover cycles. These tests use synthetic requests/sessions; they do not authorize a real tool command. Session screenshots are written under `docs/design/evidence/` and can change tracked or untracked files.

## Native Linux checks

After building the portable Linux executable:

```sh
xvfb-run -a python3 platform/linux/x11/tests/native_smoke.py dist/linux/verge
```

The test creates isolated temporary Codex metadata, compares actual window pixels after navigation clicks, and checks inactivity, hover and focus. See the [E2E report](docs/design/E2E_PORTABILITY_2026-09-10.md) for the authenticated TCP-display variant used on WSL.

## Provider and permission changes

Document the observed source schema, supported OS and what a value actually measures. Preserve the distinction between reported and derived readings. Bound filesystem reads and omit prompts, tokens and unrelated metadata from observer output. Use synthetic fixtures instead of copying a real account's files into tests.

For approval changes, verify process/session identity, one-shot decisions and disconnect/termination behavior. Keep the fail-closed behavior and full-action review intact. Read [SECURITY.md](SECURITY.md) and [Claude setup](docs/design/CLAUDE_APPROVAL_SETUP.md).

## Pull requests and bug reports

Describe the user-visible problem, final behavior, and the exact checks run. Include OS, build mode and DPI/display backend for UI work. Attach sanitized screenshots and reproducible steps; label fixture data. Explain unverified platforms instead of claiming parity.

Use the repository issue templates for ordinary bugs or feature proposals. Follow [SECURITY.md](SECURITY.md) for suspected vulnerabilities. Never attach credential files, raw session transcripts or private permission commands.
