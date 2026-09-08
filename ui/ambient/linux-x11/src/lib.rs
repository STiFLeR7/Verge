//! Thin Linux/X11 ambient shell: render + refresh only, no business logic.
//!
//! Same discipline as `ui/ambient/windows`: no webview API, no HTTP client,
//! no tool adapter reference here — turning an `AmbientState` into an
//! `OverlayContent` and handing it to the platform's `OverlaySurface` is
//! the entire job.
//!
//! **Not redesigned this pass.** `docs/design/VERGE_AMBIENT_DESIGN.md` and
//! its Windows implementation (`docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`)
//! are explicitly Windows-only for this phase. This crate still renders
//! plain text via `platform/linux/x11`'s existing `PolyText8`-based
//! drawing — it is adapted here only mechanically, to keep
//! `cargo build --workspace` green after `core::ports::OverlayContent`
//! gained a structured shape (`glyphs`/`overflow_count`) for the Windows
//! capsule. Applying the approved design to Linux/X11 is future work, not
//! done here.

use verge_core::domain::{AmbientState, Availability, UsageReading};
use verge_core::ports::{OverlayContent, StateTint, ToolGlyph};

pub fn render(state: &AmbientState) -> OverlayContent {
    let header = format!("Verge · {}", state.account.label);

    let body = match &state.availability {
        Availability::Available => match &state.most_constrained_window {
            Some(window) => match window.reading {
                UsageReading::Fraction(f) => format!("{:.0}% · {}", f * 100.0, window.name),
                UsageReading::Count(c) => format!("{c} · {}", window.name),
            },
            None => "No usage window reported".to_string(),
        },
        Availability::Unauthenticated => "Needs sign-in".to_string(),
        Availability::AccessDenied => "Access declined".to_string(),
        Availability::NotMetered => "Nothing to meter yet".to_string(),
        Availability::Unsupported { reason } => format!("Not available: {reason}"),
        Availability::Unreachable => "Retrying…".to_string(),
        Availability::Error { .. } => "Temporarily unavailable".to_string(),
    };

    OverlayContent {
        glyphs: vec![ToolGlyph {
            label: state.account.label.clone(),
            mark: '•',
            brand_color: (255, 255, 255),
            state: StateTint::Neutral,
            has_metric: false,
            dimmed: false,
            detail_lines: vec![header, body],
        }],
        overflow_count: None,
    }
}

/// Runs the ambient surface, calling `next_state` on the platform's own
/// refresh cadence to get the latest `AmbientState` to render. Blocks until
/// the surface is closed.
///
/// `#[cfg(unix)]`-gated because the X11 platform implementation it calls
/// into is: keeping `render()` above ungated means the pure formatting
/// logic stays unit-testable on every platform, exactly like
/// `ui/ambient/windows` keeps `render()` free of any Win32 dependency.
#[cfg(unix)]
pub fn run_ambient_shell(
    next_state: impl Fn() -> AmbientState + Send + 'static,
) -> std::io::Result<()> {
    use verge_core::ports::OverlaySurface;
    verge_platform_linux_x11::X11OverlaySurface::new().run(move || render(&next_state()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use verge_core::domain::{Account, ToolId};

    fn account() -> Account {
        Account {
            tool: ToolId::Codex,
            label: "Codex".into(),
            provenance: "test".into(),
        }
    }

    #[test]
    fn unauthenticated_never_shows_a_number() {
        let state = AmbientState {
            account: account(),
            availability: Availability::Unauthenticated,
            most_constrained_window: None,
        };
        let content = render(&state);
        let lines = &content.glyphs[0].detail_lines;
        assert!(lines.iter().any(|l| l.contains("Needs sign-in")));
        assert!(!lines.iter().any(|l| l.contains('%')));
    }

    #[test]
    fn unsupported_usage_shows_the_reason_never_a_fabricated_number() {
        let state = AmbientState {
            account: account(),
            availability: Availability::Unsupported {
                reason: "Codex has no local usage/quota cache".to_string(),
            },
            most_constrained_window: None,
        };
        let content = render(&state);
        let lines = &content.glyphs[0].detail_lines;
        assert!(lines.iter().any(|l| l.contains("Not available")));
        assert!(!lines.iter().any(|l| l.contains('%')));
    }
}
