//! Thin Windows ambient shell: composes `AmbientState` into presentation
//! data only, per `docs/design/VERGE_AMBIENT_DESIGN.md`. A contributor
//! reaching for a webview API, an HTTP client, or a tool adapter here is a
//! structural smell — that all belongs to `core`, `tools/`, or
//! `platform/windows`, never here.

use verge_core::domain::{AmbientState, Availability, Recency, ToolId, UsageReading};
use verge_core::ports::{OverlayContent, StateTint, ToolGlyph};

/// Density cap from `docs/design/VERGE_AMBIENT_DESIGN.md` §18: more tools
/// than this collapse into a compact "+N", never a longer visible list.
const GLYPH_CAP: usize = 4;

/// `AmbientState` -> presentation-ready glyphs. Kept separate from
/// `run_ambient_shell` so it is unit-testable without a real window. Takes
/// a slice rather than a single state so the density cap and future
/// multi-tool composition already have a real home, even though exactly
/// one real `UsageSource` (Claude Code) exists today — see
/// `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`.
pub fn render(states: &[AmbientState]) -> OverlayContent {
    let mut glyphs: Vec<ToolGlyph> = states.iter().map(render_one).collect();

    let overflow_count = if glyphs.len() > GLYPH_CAP {
        let overflow = (glyphs.len() - GLYPH_CAP) as u32;
        glyphs.truncate(GLYPH_CAP);
        Some(overflow)
    } else {
        None
    };

    OverlayContent {
        glyphs,
        overflow_count,
    }
}

fn render_one(state: &AmbientState) -> ToolGlyph {
    let (mark, brand_color, label) = identity(state.account.tool);

    let (has_metric, dimmed, detail_lines) = match &state.availability {
        Availability::Available => match &state.most_constrained_window {
            Some(window) => {
                let dimmed = matches!(window.recency, Recency::Aged { .. });
                match window.reading {
                    UsageReading::Fraction(f) => {
                        (true, dimmed, vec![format!("{:.0}% used", f * 100.0)])
                    }
                    UsageReading::Count(c) => {
                        // The domain's own window name already carries a
                        // parenthetical technical qualifier
                        // ("... (local count, no published limit)") that
                        // is correct for a diagnostic context but not for
                        // the ambient surface — see design spec §21. Strip
                        // it rather than showing it verbatim.
                        let human_name = window
                            .name
                            .split(" (local count")
                            .next()
                            .unwrap_or(&window.name);
                        (
                            false,
                            dimmed,
                            vec![
                                format!("{c} {human_name}"),
                                "No published limit".to_string(),
                            ],
                        )
                    }
                }
            }
            None => (false, false, vec!["No usage reported".to_string()]),
        },
        Availability::Unauthenticated => (false, false, vec!["Needs sign-in".to_string()]),
        Availability::AccessDenied => (false, false, vec!["Access declined".to_string()]),
        Availability::NotMetered => (false, false, vec!["Nothing to meter yet".to_string()]),
        // Never echo the raw `reason` string here: for this adapter it is
        // an OS/IO error message, exactly the kind of implementation
        // detail design spec §21 forbids in the ambient surface.
        Availability::Unsupported { .. } => (
            false,
            false,
            vec!["Not available on this device".to_string()],
        ),
        Availability::Unreachable => (false, false, vec!["Retrying…".to_string()]),
        // `diagnostic` is explicitly never shown raw to the user —
        // `docs/PRODUCT_ARCHITECTURE.md` §6.
        Availability::Error { .. } => (false, false, vec!["Temporarily unavailable".to_string()]),
    };

    ToolGlyph {
        label: label.to_string(),
        mark,
        brand_color,
        // No `ActivitySource` exists for any tool yet — always `Neutral`.
        // See the "Activity Gap" section of
        // docs/design/VERGE_AMBIENT_IMPLEMENTATION.md. Never simulated.
        state: StateTint::Neutral,
        has_metric,
        dimmed,
        detail_lines,
    }
}

/// Single-character stand-ins for each tool's real brand mark (no vector/
/// bitmap logo assets exist in this repository yet — see
/// `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`), paired with an
/// approximation of that tool's real brand color (also unverified against
/// an official source — documented, not silently asserted as exact).
/// Exhaustive over `ToolId` so every future tool already has a visual
/// identity slot without touching the render pipeline shape — only
/// `ToolId::ClaudeCode` is backed by a real, running adapter today.
fn identity(tool: ToolId) -> (char, (u8, u8, u8), &'static str) {
    match tool {
        ToolId::ClaudeCode => ('✳', (218, 119, 86), "Claude"),
        ToolId::Cursor => ('▲', (232, 211, 62), "Cursor"),
        ToolId::Codex => ('◈', (53, 214, 124), "Codex"),
        ToolId::Glm => ('◫', (77, 208, 225), "GLM"),
        ToolId::Antigravity => ('◎', (124, 155, 255), "Antigravity"),
        ToolId::Grok => ('✶', (185, 139, 255), "Grok"),
        ToolId::OpenCode => ('◇', (200, 200, 200), "OpenCode"),
    }
}

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
    verge_platform_windows::WindowsOverlaySurface::new().run(move || render(&next_states()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use verge_core::domain::{Account, Fidelity, UsageWindow};

    fn account(tool: ToolId) -> Account {
        Account {
            tool,
            label: "test".into(),
            provenance: "test".into(),
        }
    }

    fn state(availability: Availability, window: Option<UsageWindow>) -> AmbientState {
        AmbientState {
            account: account(ToolId::ClaudeCode),
            availability,
            most_constrained_window: window,
        }
    }

    #[test]
    fn unauthenticated_never_shows_a_number_or_a_metric_ring() {
        let content = render(&[state(Availability::Unauthenticated, None)]);
        let glyph = &content.glyphs[0];
        assert!(!glyph.has_metric);
        assert!(glyph
            .detail_lines
            .iter()
            .any(|l| l.contains("Needs sign-in")));
        assert!(!glyph.detail_lines.iter().any(|l| l.contains('%')));
    }

    #[test]
    fn bare_count_never_gets_a_fake_percentage_ring() {
        let window = UsageWindow {
            name: "messages on 2026-09-07 (local count, no published limit)".into(),
            reading: UsageReading::Count(42),
            resets_at: None,
            fidelity: Fidelity::Derived,
            recency: Recency::Live {
                as_of: SystemTime::now(),
            },
        };
        let content = render(&[state(Availability::Available, Some(window))]);
        let glyph = &content.glyphs[0];
        assert!(
            !glyph.has_metric,
            "a bare count must never render as a percentage-fill ring"
        );
        assert!(glyph.detail_lines.iter().any(|l| l.contains("42")));
        assert!(glyph.detail_lines.iter().any(|l| l == "No published limit"));
        // The raw technical qualifier must never reach the UI verbatim.
        assert!(!glyph.detail_lines.iter().any(|l| l.contains("local count")));
    }

    #[test]
    fn fraction_reading_does_get_a_metric_ring() {
        let window = UsageWindow {
            name: "5h window".into(),
            reading: UsageReading::Fraction(0.73),
            resets_at: None,
            fidelity: Fidelity::Official,
            recency: Recency::Live {
                as_of: SystemTime::now(),
            },
        };
        let content = render(&[state(Availability::Available, Some(window))]);
        assert!(content.glyphs[0].has_metric);
        assert!(content.glyphs[0]
            .detail_lines
            .iter()
            .any(|l| l.contains("73%")));
    }

    #[test]
    fn aged_reading_is_flagged_dimmed_not_given_extra_text() {
        let window = UsageWindow {
            name: "5h window".into(),
            reading: UsageReading::Fraction(0.5),
            resets_at: None,
            fidelity: Fidelity::Official,
            recency: Recency::Aged {
                as_of: SystemTime::UNIX_EPOCH,
            },
        };
        let content = render(&[state(Availability::Available, Some(window))]);
        assert!(content.glyphs[0].dimmed);
        assert_eq!(
            content.glyphs[0].detail_lines.len(),
            1,
            "dimming is a material treatment, not more text"
        );
    }

    #[test]
    fn unsupported_reason_is_never_echoed_verbatim() {
        let content = render(&[state(
            Availability::Unsupported {
                reason: "os error 5 (Access is denied.)".into(),
            },
            None,
        )]);
        let lines = &content.glyphs[0].detail_lines;
        assert!(!lines.iter().any(|l| l.contains("os error")));
        assert!(lines.iter().any(|l| l.contains("Not available")));
    }

    #[test]
    fn no_activity_source_yet_means_every_glyph_is_neutral() {
        let content = render(&[state(Availability::Available, None)]);
        assert_eq!(content.glyphs[0].state, StateTint::Neutral);
    }

    #[test]
    fn overflow_caps_the_visible_glyph_count() {
        let states: Vec<AmbientState> = (0..6)
            .map(|_| state(Availability::Unauthenticated, None))
            .collect();
        let content = render(&states);
        assert_eq!(content.glyphs.len(), GLYPH_CAP);
        assert_eq!(content.overflow_count, Some(2));
    }

    #[test]
    fn no_overflow_when_within_cap() {
        let states: Vec<AmbientState> = (0..2)
            .map(|_| state(Availability::Unauthenticated, None))
            .collect();
        let content = render(&states);
        assert_eq!(content.overflow_count, None);
    }
}
