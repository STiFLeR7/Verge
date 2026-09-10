//! Read-only diagnostic: reports verified activity, never session names or credentials.
#[cfg(windows)]
fn main() {
    use verge_core::{
        domain::{project_ambient_state, Account, Availability, ToolId, UsageSnapshot},
        ports::ActivitySource,
    };
    use verge_platform_windows::WindowsProcessProbe;
    use verge_tool_claude::{default_sessions_path, ClaudeActivitySource};
    let sessions = ClaudeActivitySource::new(default_sessions_path().unwrap(), WindowsProcessProbe)
        .sessions()
        .expect("session metadata readable");
    let fallback = UsageSnapshot {
        account: Account {
            tool: ToolId::ClaudeCode,
            label: "Claude".into(),
            provenance: "local".into(),
        },
        availability: Availability::NotMetered,
        windows: vec![],
    };
    let snapshot = verge_tool_claude::read_limits(&fallback.account).unwrap_or(fallback);
    let state = project_ambient_state(&snapshot).with_sessions(Some(sessions));
    for window in &state.usage_windows {
        println!(
            "{}: {:?}; reset present: {}",
            window.name,
            window.reading,
            window.resets_at.is_some()
        );
    }
    let output = verge_ui_ambient_windows::render(std::slice::from_ref(&state));
    println!(
        "Verified sessions: {}; activity: {:?}; visible glyphs: {}",
        state.sessions.as_ref().unwrap().len(),
        state.activity,
        output.glyphs.len()
    );
}
#[cfg(not(windows))]
fn main() {}
