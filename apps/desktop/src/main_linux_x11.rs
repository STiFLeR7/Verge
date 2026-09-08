//! Composition root: wires `core` + the Codex tool adapter + Linux/X11
//! platform capabilities + the ambient UI shell into one running Linux
//! process. The second vertical slice — proves the same contracts
//! (`UsageSource`, `CredentialStore`, `OverlaySurface`) that `main.rs`
//! (Claude Code + Windows) uses also compose cleanly for a different tool
//! on a different OS.
//!
//! Everything below is `#[cfg(unix)]`-gated so this binary target still
//! compiles (as a no-op) on Windows, matching how `platform/linux/x11`
//! itself gates its X11-specific code — the goal is that
//! `cargo build --workspace` keeps succeeding on the Windows side of this
//! repo without needing X11 headers, exactly as it does today.

#[cfg(unix)]
fn main() -> std::io::Result<()> {
    use verge_core::application::{select_product_mode, ProductMode};
    use verge_core::domain::{
        project_ambient_state, Account, AmbientState, Capability, CapabilityLevel,
        CapabilityProfile, ToolId,
    };
    use verge_core::ports::UsageSource;
    use verge_platform_linux_x11::LinuxCredentialStore;
    use verge_tool_codex::CodexUsageSource;

    // Declared `Full` only because a real platform implementation exists
    // and has been live-tested (see docs/design/SECOND_VERTICAL_SLICE.md) —
    // not asserted by default the way `main.rs` does for Windows.
    let mut capabilities = CapabilityProfile::new();
    capabilities.set(Capability::AmbientOverlay, CapabilityLevel::Full);
    match select_product_mode(&capabilities) {
        ProductMode::Full => eprintln!("[verge] product mode: FULL"),
        other => eprintln!("[verge] product mode: {other:?} (unexpected on this build)"),
    }

    let account = Account {
        tool: ToolId::Codex,
        label: "Codex".to_string(),
        provenance: "~/.codex (discovered via CODEX_HOME/HOME)".to_string(),
    };

    let usage_source = CodexUsageSource::new(LinuxCredentialStore::discover());

    let next_state = move || -> AmbientState {
        let snapshot = usage_source.fetch(&account).unwrap_or_else(|availability| {
            verge_core::domain::UsageSnapshot {
                account: account.clone(),
                availability,
                windows: vec![],
            }
        });
        project_ambient_state(&snapshot)
    };

    verge_ui_ambient_linux_x11::run_ambient_shell(next_state)
}

#[cfg(not(unix))]
fn main() {
    eprintln!("verge-linux-x11 is a Linux/X11-only binary; nothing to run on this platform.");
}
