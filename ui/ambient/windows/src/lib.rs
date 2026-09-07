//! Thin Windows ambient shell: render + refresh only, no business logic.
//!
//! A contributor reaching for a webview API, an HTTP client, or a tool
//! adapter here is a structural smell — that all belongs to `core`,
//! `tools/`, or `platform/windows`, never here. This crate's only job is to
//! turn an `AmbientState` into an `OverlayContent` and hand it to the
//! platform's `OverlaySurface`.

use verge_core::domain::{AmbientState, Availability, UsageReading};
use verge_core::ports::{OverlayContent, OverlaySurface};
use verge_platform_windows::WindowsOverlaySurface;

/// Pure formatting: `AmbientState` -> the lines the surface renders. Kept
/// separate from `run_ambient_shell` so it is unit-testable without a real
/// window.
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

    OverlayContent { lines: vec![header, body] }
}

/// Runs the ambient surface, calling `next_state` on the platform's own
/// refresh cadence to get the latest `AmbientState` to render. Blocks until
/// the surface is closed.
pub fn run_ambient_shell(next_state: impl Fn() -> AmbientState + Send + 'static) -> std::io::Result<()> {
    WindowsOverlaySurface::new().run(move || render(&next_state()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use verge_core::domain::{Account, Fidelity, Recency, ToolId, UsageWindow};
    use std::time::SystemTime;

    fn account() -> Account {
        Account { tool: ToolId::ClaudeCode, label: "Claude Code".into(), provenance: "test".into() }
    }

    #[test]
    fn unauthenticated_never_shows_a_number() {
        let state = AmbientState {
            account: account(),
            availability: Availability::Unauthenticated,
            most_constrained_window: None,
        };
        let content = render(&state);
        assert!(content.lines.iter().any(|l| l.contains("Needs sign-in")));
        assert!(!content.lines.iter().any(|l| l.contains('%')));
    }

    #[test]
    fn available_with_count_renders_the_count() {
        let state = AmbientState {
            account: account(),
            availability: Availability::Available,
            most_constrained_window: Some(UsageWindow {
                name: "messages on 2026-09-07".into(),
                reading: UsageReading::Count(42),
                resets_at: None,
                fidelity: Fidelity::Derived,
                recency: Recency::Live { as_of: SystemTime::now() },
            }),
        };
        let content = render(&state);
        assert!(content.lines.iter().any(|l| l.contains("42")));
    }
}
