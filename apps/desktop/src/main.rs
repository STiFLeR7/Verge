//! Composition root: wires `core` + one tool adapter + platform capabilities
//! + the ambient UI shell into one running Windows process. This is the
//! first vertical slice per docs/PRODUCT_ARCHITECTURE.md §16 step — it
//! deliberately does not attempt every tool, every OS, or the detail/
//! settings surface yet.
//!
//! `#[cfg(windows)]`-gated (see `main_linux_x11.rs` for why: this binary
//! and the Linux/X11 one share one `apps/desktop` package, hence one
//! `[dependencies]` table, so each binary's real body must be gated to
//! keep the *other* platform's build clean).

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    use verge_core::application::{select_product_mode, ProductMode};
    use verge_core::domain::{
        project_ambient_state, Account, AmbientState, Capability, CapabilityLevel,
        CapabilityProfile, ToolId,
    };
    use verge_core::ports::UsageSource;
    use verge_platform_windows::WindowsCredentialStore;
    use verge_tool_claude::{default_stats_cache_path, ClaudeCodeUsageSource};

    // Validated directly by the overlay-capability spike (SPIKE_RESULTS.md
    // §2, §8): Windows delivers a `FULL` ambient overlay. This is the only
    // capability declared today because it is the only one a real platform
    // implementation exists for — declaring more here would be asserting a
    // capability nothing has measured, exactly what CapabilityProfile
    // exists to prevent.
    let mut capabilities = CapabilityProfile::new();
    capabilities.set(Capability::AmbientOverlay, CapabilityLevel::Full);
    match select_product_mode(&capabilities) {
        ProductMode::Full => eprintln!("[verge] product mode: FULL"),
        other => eprintln!("[verge] product mode: {other:?} (unexpected on this build)"),
    }

    let account = Account {
        tool: ToolId::ClaudeCode,
        label: "Claude Code".to_string(),
        provenance: "~/.claude (discovered via USERPROFILE)".to_string(),
    };

    let stats_cache_path = default_stats_cache_path().ok_or_else(|| {
        std::io::Error::other("could not resolve a home directory (USERPROFILE/HOME unset)")
    })?;
    let usage_source =
        ClaudeCodeUsageSource::new(WindowsCredentialStore::discover(), stats_cache_path);

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

    verge_ui_ambient_windows::run_ambient_shell(next_state)
}

#[cfg(not(windows))]
fn main() {
    eprintln!("verge is a Windows-only binary; nothing to run on this platform.");
}
