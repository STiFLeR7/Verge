//! Windows composition for the shared native presentation.
#[cfg(windows)]
use verge_core::{
    domain::{AmbientState, ToolId},
    ports::StateTint,
};
pub use verge_ui_ambient_shared::render;
#[cfg(windows)]
use verge_ui_ambient_shared::render_requested;
/// Runs the ambient surface, calling `next_states` on the platform's own
/// refresh cadence to get the latest states to render. Blocks until the
/// surface is closed.
///
/// `#[cfg(windows)]`-gated so this crate — and anything depending on it —
/// still compiles on non-Windows targets; keeping `render()` above ungated
/// means the pure formatting logic stays unit-testable on every platform
/// (established in the second vertical slice, see
/// `docs/design/SECOND_VERTICAL_SLICE.md`).
#[cfg(windows)]
pub fn run_ambient_shell(
    next_states: impl Fn() -> Vec<AmbientState> + Send + 'static,
) -> std::io::Result<()> {
    use verge_core::ports::OverlaySurface;
    let open = std::env::args().any(|arg| arg == "--open");
    verge_platform_windows::WindowsOverlaySurface::new()
        .run(move || render_requested(&next_states(), open))
}

#[cfg(windows)]
pub fn run_with_permissions(
    next_states: impl Fn() -> Vec<AmbientState> + Send + 'static,
    permissions: std::sync::Arc<dyn verge_core::ports::PermissionService>,
) -> std::io::Result<()> {
    let reader = permissions.clone();
    let open = std::env::args().any(|arg| arg == "--open");
    verge_platform_windows::WindowsOverlaySurface::new().run_with_permissions(
        move || {
            let states = next_states();
            let mut content = render_requested(&states, open);
            if let Some(request) = reader.pending() {
                if !content.glyphs.iter().any(|g| g.label == "Claude") {
                    if let Some(state) =
                        states.iter().find(|s| s.account.tool == ToolId::ClaudeCode)
                    {
                        if content.glyphs.len() == 4 {
                            content.glyphs.pop();
                            content.overflow_count = Some(content.overflow_count.unwrap_or(0) + 1);
                        }
                        content.glyphs.insert(
                            0,
                            verge_ui_ambient_shared::render_requested(
                                std::slice::from_ref(state),
                                true,
                            )
                            .glyphs
                            .remove(0),
                        );
                    }
                }
                if let Some(g) = content.glyphs.iter_mut().find(|g| g.label == "Claude") {
                    g.state = StateTint::Waiting;
                    g.activity_label = Some("Needs approval".into());
                    g.permission = Some(request);
                }
            }
            content
        },
        permissions,
    )
}
