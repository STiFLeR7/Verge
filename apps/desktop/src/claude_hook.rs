//! Dedicated synchronous PermissionRequest command. Failures always emit deny.
#[cfg(windows)]
fn main() {
    use std::io::{Read, Write};
    let mut raw = Vec::new();
    let allowed = (|| -> Option<bool> {
        std::io::stdin().take(32769).read_to_end(&mut raw).ok()?;
        let input = verge_tool_claude::parse_permission(&raw)?;
        let pid = verge_platform_windows::claude_ancestor(std::process::id())?;
        if !verge_tool_claude::permission_session_matches(
            &input.session,
            pid,
            &verge_platform_windows::WindowsProcessProbe,
        ) {
            return None;
        }
        let approved = verge_platform_windows::request_permission(
            &input.session,
            &input.tool,
            &input.detail,
            &input.cwd,
        );
        Some(
            approved
                && verge_tool_claude::permission_session_matches(
                    &input.session,
                    pid,
                    &verge_platform_windows::WindowsProcessProbe,
                ),
        )
    })()
    .unwrap_or(false);
    // Claude may have exited while its permission was pending.
    let _ = writeln!(
        std::io::stdout(),
        "{}",
        verge_tool_claude::permission_output(allowed)
    );
}
#[cfg(not(windows))]
fn main() {}
