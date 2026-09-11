# Verge v1.0.0 verification ledger

This ledger records release evidence at the exact candidate revision. `OPEN` is not a pass and blocks the stable release when the support contract requires it.

| Gate | Result | Current evidence or required action |
|---|---|---|
| Scope and platform tiers | PASS | `V1_SUPPORT_CONTRACT.md`, README, and portability matrix agree. |
| Distribution license | PASS | The owner selected Apache-2.0; Cargo metadata, contribution terms, and packaged notices use the same terms. |
| Windows automated native contract | PASS | Portable CI run `34565124903` at `5e3a925`; later candidate must repeat it. |
| Windows physical multi-monitor pass | OPEN | Requires 125% primary plus 100%/150% secondary evidence. |
| Linux X11/XWayland native contract | PASS | Nested Xephyr/Xvfb verifies navigation, inactivity, focus, EWMH, work area, input shape, and RandR. |
| macOS automated AppKit contract | PASS | Portable CI run `34565124903` verifies build, UI contract, navigation/inactivity state, screen notification, signing, and package. |
| macOS real-host interaction | OPEN | Requires macOS 13+ Apple-silicon Spaces/fullscreen/hover/display evidence. |
| Claude synthetic security boundary | PASS | Workspace Rust and Node contracts cover identity/session/nonce/timeout/privacy behavior. |
| Claude live-host install/approve/deny/offline/uninstall | OPEN | Requires sanitized real Claude reload evidence. |
| Windows 8-hour soak | OPEN | Run `scripts/soak.ps1` against the signed candidate. |
| Linux 2-hour soak | OPEN | Run `scripts/soak.sh` against the signed candidate under X11/XWayland. |
| macOS 2-hour soak | OPEN | Run `scripts/soak.sh` against the signed/notarized app executable. |
| Signing credentials and signed artifacts | OPEN | Configure the GitHub `release` environment secrets and run the tag workflow. |
| Checksums and archive validation | PASS (synthetic) | `scripts/check-release.ps1` accepts complete fixtures and rejects unlicensed archives; repeat on final signed artifacts. |

## Candidate commands

```powershell
cargo fmt --all --check
cargo test --workspace
cargo check --workspace --all-targets
node scripts/integrations/test.mjs
node scripts/claude-signal.test.cjs
./scripts/soak.ps1 -Executable ./dist/windows/verge.exe -DurationMinutes 480 -OutputPath ./v1-windows-soak.csv
```

```sh
scripts/soak.sh dist/linux/verge 7200 60 v1-linux-soak.csv
scripts/soak.sh dist/Verge.app/Contents/MacOS/verge-macos 7200 60 v1-macos-soak.csv
```

Soak CSVs contain timestamps and process metrics only. Store reviewed, sanitized results with the candidate evidence; do not commit account metadata, prompts, commands, session IDs, or credentials.
