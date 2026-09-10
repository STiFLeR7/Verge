//! Local metadata inspection; never reads credentials; Codex uses bounded rollout tails and prints metadata only.
#[cfg(windows)]
fn main() {
    use verge_core::ports::ActivitySource;
    let source = verge_tool_claude::ClaudeActivitySource::new(
        verge_tool_claude::default_sessions_path().unwrap(),
        verge_platform_windows::WindowsProcessProbe,
    );
    if let Some(path) = verge_tool_codex::local::default_sessions_path() {
        let state = verge_tool_codex::local::read_state(&path);
        let sessions = state.sessions.unwrap_or_default();
        println!("Recent Codex sessions: {}", sessions.len());
        for session in sessions {
            println!(
                "Codex activity: {:?}; intelligence: {:?}",
                session.state, session.intelligence
            );
        }
    }
    match source.sessions() {
        Ok(sessions) => {
            println!("Verified Claude sessions: {}", sessions.len());
            for session in sessions {
                println!(
                    "Activity: {:?}; intelligence: {:?}",
                    session.state, session.intelligence
                );
            }
        }
        Err(_) => println!("Session metadata unavailable"),
    }
}
#[cfg(not(windows))]
fn main() {}
