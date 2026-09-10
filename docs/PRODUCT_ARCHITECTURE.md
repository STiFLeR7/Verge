# Verge architecture

This is the current architecture overview. Section numbers used by older repository references are retained where useful; earlier design records can describe capabilities that are still planned.

## 1. Product

Verge presents local AI-agent activity, reported account usage and session context in a small native desktop surface. It does not submit prompts or run an agent. Windows Claude approval is an explicit, separate decision path.

## Source boundaries

```text
Tool-owned local data
        |
 tools/claude + tools/codex + observer metadata
        |
 core/ domain and application rules
        |
 ui/ambient/shared presentation
        |
 Win32 surface | X11 surface | Rust JSON bridge -> AppKit
```

`apps/desktop` composes the running application. `platform/windows` also owns Windows discovery, process verification and the approval pipe. `platform/linux/x11` owns native Linux window/input behavior. `ui/ambient/macos` is a dependency-free Swift/AppKit package consuming the sibling Rust `verge-state` executable.

## 5. Capabilities and missing data

The domain distinguishes availability, fidelity and freshness. A reported fraction, a count without a published limit and an unavailable value are different states. Activity and quota are independent. Context capacity belongs to an individual session rather than an account usage window.

Platform implementations are not interchangeable in completeness. Windows has the most complete surface and the only approval broker. Linux requires X11/XWayland. The AppKit shell is experimental. The capability model does not imply that every planned fallback or OS feature is implemented; consult the [platform matrix](design/PORTABILITY.md).

## 11. Local data and trust

Adapters own schema interpretation and bounded local reads. Credential, transcript and command content must not leak into observer output or diagnostic fixtures. Source files such as Codex rollouts may contain conversation text even though the adapter extracts selected metadata.

Windows discovery can install missing observer files. The optional Claude installer modifies settings and status-line integration with backups. The broker handles permission actions separately from passive activity. Its per-user checks do not form a sandbox against malicious same-user processes. [Security details](../SECURITY.md).

## 13. Presentation and native surfaces

Shared Rust presentation maps domain state to names, limit rows, session details and attention priority. Native shells implement their own layout and input. Windows privately loads bundled Inter; Linux still has legacy core-font rendering; macOS bundles Inter with its shell.

The Swift bridge exposes presentation JSON and transient view identifiers. It is a local rendering boundary, not an authorization API. The proposed Tauri detail/settings surface in [ADR 0004](adr/0004-shared-tauri-detail-surface.md) has not been built.

## 16. Composition and verification

Cargo target-specific dependencies keep Win32 and X11 imports on their intended OS. Shared domain/presentation tests run across targets. Rust cross-checks cannot validate Swift compilation, screen geometry or native input. Separate fixtures exercise Windows navigation/approval and Linux native rendering/input. See [contribution checks](../CONTRIBUTING.md) and [E2E evidence](design/E2E_PORTABILITY_2026-09-10.md).

## 20. Packaging and distribution

Build scripts produce host-specific Windows ZIP, Linux tarball and experimental macOS app/ZIP outputs. These are development packages. Linux depends on compatible host libraries; macOS packaging uses ad-hoc signing. Public licensing, Developer ID/notarized distribution and automatic updates have not been established. The [repository license](../LICENSE) and [third-party notices](../THIRD_PARTY_NOTICES.md) remain authoritative.
