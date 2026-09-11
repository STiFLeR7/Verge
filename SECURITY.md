# Security and privacy

Verge is pre-release software. There is no published supported-version or security-update schedule. Check the [platform status](docs/design/PORTABILITY.md) before relying on a capability.

## Reporting a vulnerability

Do not publish credentials, private transcripts or a working permission-bypass exploit in an issue. Use a private reporting channel provided by the repository maintainer. If none is available, open a minimal issue requesting private contact without including exploit details. A dedicated security mailbox and enabled GitHub private reporting are not assumed by this document.

Include the affected revision, OS, component, reproducible steps with synthetic inputs, expected/actual behavior and potential impact. Avoid real account data.

## Local data access

- The Windows Claude fallback reads `.claude/.credentials.json` and the statistics cache. Credential contents must not enter logs or persisted observer output.
- Claude status-line/hooks supply selected usage, context and activity metadata. Codex metadata is extracted from bounded rollout reads; those source files may also contain conversation text. This is not a guarantee that prompt bytes are never read from disk.
- Windows discovery checks known installation locations and process names and can write missing observer integration files. The optional Claude installer changes settings/status-line files and creates backups. Its uninstall mode removes only Verge-owned hook commands and the marked observer line; it never restores whole configuration files.
- Permission review receives the proposed tool action so the user can inspect it. Treat screenshots of this view as potentially sensitive.
- The current app has no telemetry or separate provider sign-in flow. Do not infer that local data is harmless to share.

## Claude approval boundary

Direct approval is implemented only for Claude on Windows. The helper and broker use a per-user pipe ACL, executable identity checks, process-start-time binding, a request nonce and one-shot session-bound decisions. An unavailable broker, failed validation, disconnect or response timeout returns deny when the helper runs.

These controls are not isolation from malicious software already running as the same OS user. Parent-process spoofing and readable local session metadata limit what ancestor correlation can prove. Do not weaken the checks or describe them as a same-user sandbox.

Use the [setup and recovery guide](docs/design/CLAUDE_APPROVAL_SETUP.md) to install or remove the gate. A passive activity observer is not a permission broker. Codex, Linux and macOS have no direct approval path in this implementation.

## Verification scope

The [E2E report](docs/design/E2E_PORTABILITY_2026-09-10.md) records native fixture and transport checks. The [v1 verification ledger](docs/releases/V1_VERIFICATION.md) keeps hardware, live-host, soak, signing, and licensing gates open until exact-revision evidence exists. Neither document is a formal security audit.
