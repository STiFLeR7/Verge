# Claude approval hook: installation and trust boundary

The metadata observer reports activity. It does not return permission decisions. The synchronous `verge-claude-hook.exe` receives Claude's `PermissionRequest` input and communicates with Verge's native broker to return one allow/deny decision. Both registrations are required for the complete experience.

## Install

Build `cargo build -p verge-desktop --bin verge --bin verge-claude-hook`, start `target/debug/verge.exe`, then run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install-claude-signals.ps1 -BinaryDirectory D:/Verge/target/debug
```

The binary directory defaults to this checkout's `target/debug`; provide the directory explicitly for another build. Both executables must be present together. Run Verge from that same directory because client/server image-path checks bind the pair. The installer checks files before modifying settings, preserves other hooks/status-line output, retains original backups, and registers exactly one matching synchronous decision command with a 130-second hook timeout (longer than the helper's 120-second response deadline). It refuses to add a second helper from a different directory. This installer currently supports the existing `statusline-command.js` integration with its guarded source anchor; it is not a generic installer for arbitrary status-line implementations.

**Once installed, Verge must be running before Claude requests permission.** If the helper executes but the broker is absent/unreachable, identity validation fails, the connection is lost or the request times out, it returns deny. There is no automatic fallback to approval or to Claude's normal prompt. This only applies when Claude invokes `PermissionRequest`, not to calls already allowed by Claude's permission policy. Start Verge and retry the call. Restart/reload Claude after hook configuration changes so it uses the new registration.

To return to Claude-managed decisions, remove only the command invoking `verge-claude-hook.exe` from `.claude/settings.json` under `hooks.PermissionRequest`, preserving the metadata observer and other user hooks, then reload Claude. Do not restore an old whole-settings backup over newer unrelated settings. If the binary itself cannot launch, Claude controls hook-error handling; that case is not guaranteed to emit the helper's deny JSON.

## Trust boundary

The pipe ACL isolates Windows users. The first-instance flag, image-path checks, process-start-time checks, request nonce and request/session binding protect against accidental cross-session responses, PID reuse and stale/mismatched decisions. They do not establish an isolation boundary against arbitrary malicious code already running as the same Windows user. Such code may spoof a parent PID when creating a process, read that user's Claude session metadata, or modify user-writable executables/settings. Ancestor correlation is not cryptographic proof that Claude intentionally launched a request.

Treat local same-user code as trusted for this integration. No stronger provenance claim is made, and this clarification does not weaken existing checks or introduce automatic approval. Approve remains an explicit user decision on the bound request. Defending against a compromised user account requires a different security boundary and is outside this integration's scope.

Reference: [Claude hook documentation](https://code.claude.com/docs/en/hooks#permissionrequest).

## Review correction: Pi/Kilo packaging

The previous installed layout was functional: `install_observer` already emitted the shared module as `verge-observer-lib.mjs`. The discrepancy was between the repository filename (`observer.mjs`) and that installed name, not an absent dependency in the shipped Windows installation. The source file is now named `verge-observer-lib.mjs` too. Node tests resolve the real repository dependency without renaming it, and a Rust test verifies the actual installed Pi/Kilo layout and preservation of existing user files.
