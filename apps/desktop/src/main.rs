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
    use verge_core::ports::{ActivitySource, UsageSource};
    use verge_platform_windows::{WindowsCredentialStore, WindowsProcessProbe};
    use verge_tool_claude::{
        default_sessions_path, default_stats_cache_path, ClaudeActivitySource,
        ClaudeCodeUsageSource,
    };

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

    let activity_source = ClaudeActivitySource::new(
        default_sessions_path()
            .ok_or_else(|| std::io::Error::other("could not resolve Claude sessions"))?,
        WindowsProcessProbe,
    );
    // Independent cadences: usage is cached; lightweight local activity is refreshed each tick.
    let cache =
        std::sync::Mutex::new(None::<(std::time::Instant, verge_core::domain::UsageSnapshot)>);
    let reminders = std::sync::Mutex::new((
        verge_core::application::UsageReminders::default(),
        None::<(std::time::Instant, String)>,
    ));
    let codex_path = verge_tool_codex::local::default_sessions_path();
    let codex_cache = std::sync::Mutex::new(None::<(std::time::Instant, AmbientState)>);
    let detected = verge_platform_windows::discover_tools();
    verge_platform_windows::install_observers(&detected);
    let discovery = std::sync::Mutex::new((std::time::Instant::now(), detected));
    let next_states = move || -> Vec<AmbientState> {
        let mut cache = cache.lock().unwrap();
        if cache
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() >= std::time::Duration::from_secs(60))
        {
            let snapshot = usage_source.fetch(&account).unwrap_or_else(|availability| {
                verge_core::domain::UsageSnapshot {
                    account: account.clone(),
                    availability,
                    windows: vec![],
                }
            });
            *cache = Some((std::time::Instant::now(), snapshot));
        }
        let limits = verge_tool_claude::read_limits(&account);
        let snapshot = limits.as_ref().unwrap_or(&cache.as_ref().unwrap().1);
        let mut reminders = reminders.lock().unwrap();
        if let Some(message) = reminders.0.update(snapshot, std::time::SystemTime::now()) {
            reminders.1 = Some((std::time::Instant::now(), message));
        }
        let mut state =
            project_ambient_state(snapshot).with_sessions(activity_source.sessions().ok());
        state.reminder = reminders
            .1
            .as_ref()
            .filter(|(at, _)| at.elapsed() < std::time::Duration::from_secs(12))
            .map(|(_, message)| message.clone());
        let mut states = vec![state];
        if let Some(path) = &codex_path {
            let mut cached = codex_cache.lock().unwrap();
            if cached
                .as_ref()
                .is_none_or(|(at, _)| at.elapsed() >= std::time::Duration::from_secs(3))
            {
                *cached = Some((
                    std::time::Instant::now(),
                    verge_tool_codex::local::read_state(path),
                ));
            }
            states.push(cached.as_ref().unwrap().1.clone());
        }
        let mut discovered = discovery.lock().unwrap();
        if discovered.0.elapsed() >= std::time::Duration::from_secs(30) {
            *discovered = (
                std::time::Instant::now(),
                verge_platform_windows::discover_tools(),
            );
            verge_platform_windows::install_observers(&discovered.1);
        }
        for known in &discovered.1 {
            if let Some(existing) = states
                .iter_mut()
                .find(|s| s.account.tool == known.account.tool)
            {
                if existing.sessions.as_ref().is_none_or(|v| v.is_empty())
                    && existing.usage_windows.is_empty()
                {
                    *existing = known.clone();
                }
            } else {
                states.push(known.clone());
            }
        }
        for observed in verge_platform_windows::observed_tools() {
            if let Some(existing) = states
                .iter_mut()
                .find(|s| s.account.tool == observed.account.tool)
            {
                *existing = observed;
            } else {
                states.push(observed);
            }
        }
        states
    };

    let permissions = verge_platform_windows::WindowsPermissions::start(|session, pid| {
        verge_tool_claude::permission_session_matches(session, pid, &WindowsProcessProbe)
    });
    match permissions {
        Ok(service) => verge_ui_ambient_windows::run_with_permissions(next_states, service),
        Err(_) => {
            eprintln!("[verge] Direct approvals unavailable; requests will not be approved.");
            verge_ui_ambient_windows::run_ambient_shell(next_states)
        }
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
fn main() -> std::io::Result<()> {
    linux::run()
}

#[cfg(target_os = "macos")]
fn main() -> std::io::Result<()> {
    let shim = std::env::current_exe()?.with_file_name("verge-macos");
    let status = std::process::Command::new(shim).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("macOS surface exited unsuccessfully"))
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn main() {
    eprintln!("Verge does not support this operating system.");
    std::process::exit(1);
}
