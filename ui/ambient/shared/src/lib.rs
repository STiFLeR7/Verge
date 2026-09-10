//! Shared native ambient presentation: composes `AmbientState` into presentation
//! data only, per `docs/design/VERGE_AMBIENT_DESIGN.md`. A contributor
//! reaching for a webview API, an HTTP client, or a tool adapter here is a
//! structural smell — that all belongs to `core`, `tools/`, or
//! `platform/windows`, never here.

use std::time::SystemTime;
use verge_core::domain::{
    ActivityState, AmbientState, Availability, Recency, ToolId, UsageReading,
};
use verge_core::ports::{Metric, OverlayContent, StateTint, ToolGlyph};

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
    let mut glyphs: Vec<ToolGlyph> = states
        .iter()
        .filter(|s| {
            !matches!(s.availability, Availability::NotMetered)
                || s.sessions.as_ref().is_some_and(|items| !items.is_empty())
        })
        .filter(|s| {
            let meaningful = s.reminder.is_some();
            meaningful
                || s.sessions.as_ref().is_none_or(|sessions| {
                    sessions.iter().any(|session| {
                        matches!(
                            session.state,
                            ActivityState::Working
                                | ActivityState::WaitingOnUser
                                | ActivityState::Unknown
                        ) || (matches!(
                            session.state,
                            ActivityState::Completed | ActivityState::Stopped
                        ) && session.since.elapsed().is_ok_and(|age| {
                            age.as_secs() < verge_core::domain::COMPLETED_GRACE_SECONDS
                        }))
                    })
                })
        })
        .map(render_one)
        .collect();

    glyphs.sort_by_key(|g| std::cmp::Reverse(tool_priority(g)));
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

    let (metric, dimmed, detail_lines) = match &state.availability {
        Availability::Available => match &state.most_constrained_window {
            Some(window) => {
                let dimmed = matches!(window.recency, Recency::Aged { .. });
                match window.reading {
                    UsageReading::Fraction(f) => (
                        Metric::Fraction(f.clamp(0.0, 1.0) as f32),
                        dimmed,
                        vec![format!("{:.0}% used", f * 100.0)],
                    ),
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
                            Metric::Neutral,
                            dimmed,
                            vec![
                                format!("{c} {human_name}"),
                                "No published limit".to_string(),
                            ],
                        )
                    }
                }
            }
            None => (Metric::None, false, vec!["No usage reported".to_string()]),
        },
        Availability::Unauthenticated => (Metric::None, false, vec!["Needs sign-in".to_string()]),
        Availability::AccessDenied => (Metric::None, false, vec!["Access declined".to_string()]),
        Availability::NotMetered => (
            Metric::None,
            false,
            vec!["Nothing to meter yet".to_string()],
        ),
        // Never echo the raw `reason` string here: for this adapter it is
        // an OS/IO error message, exactly the kind of implementation
        // detail design spec §21 forbids in the ambient surface.
        Availability::Unsupported { .. } => (
            Metric::None,
            false,
            vec!["Not available on this device".to_string()],
        ),
        Availability::Unreachable => (Metric::None, false, vec!["Retrying…".to_string()]),
        // `diagnostic` is explicitly never shown raw to the user —
        // `docs/PRODUCT_ARCHITECTURE.md` §6.
        Availability::Error { .. } => (
            Metric::None,
            false,
            vec!["Temporarily unavailable".to_string()],
        ),
    };

    let mut detail_lines = detail_lines;
    if let Some(reset) = state
        .most_constrained_window
        .as_ref()
        .and_then(|w| w.resets_at)
    {
        if let Ok(remaining) = reset.duration_since(std::time::SystemTime::now()) {
            let minutes = remaining.as_secs().div_ceil(60);
            detail_lines.push(if minutes < 60 {
                format!("Resets in {minutes} min")
            } else {
                format!("Resets in {}h {}m", minutes / 60, minutes % 60)
            });
        }
    }
    let activity = if matches!(
        state.activity,
        ActivityState::Completed | ActivityState::Stopped
    ) && state.sessions.as_ref().is_some_and(|sessions| {
        sessions
            .iter()
            .filter(|s| matches!(s.state, ActivityState::Completed | ActivityState::Stopped))
            .all(|s| {
                s.since
                    .elapsed()
                    .is_ok_and(|age| age.as_secs() >= verge_core::domain::COMPLETED_GRACE_SECONDS)
            })
    }) {
        ActivityState::RecentlyIdle
    } else {
        state.activity
    };
    let tint = match activity {
        ActivityState::Working => StateTint::Working,
        ActivityState::WaitingOnUser => StateTint::Waiting,
        ActivityState::Completed => StateTint::Completed,
        ActivityState::Stopped => StateTint::Stopped,
        _ => StateTint::Neutral,
    };
    let activity_label = match activity {
        ActivityState::Working => Some("Working"),
        ActivityState::WaitingOnUser => Some("Needs you"),
        ActivityState::Completed => Some("Completed"),
        ActivityState::Stopped => Some("Stopped / error"),
        ActivityState::RecentlyIdle => Some("Idle"),
        ActivityState::Unknown => None,
        ActivityState::Disconnected => Some("Disconnected".into()),
    }
    .map(str::to_string);
    if let Some(sessions) = &state.sessions {
        if let Some(session) = sessions.iter().find(|s| s.state == state.activity) {
            let mut context = vec![session.name.clone()];
            if let Some(reason) = &session.waiting_for {
                context.push(reason.clone());
            }
            context.extend(detail_lines);
            detail_lines = context;
        }
    }
    let sessions = if matches!(state.account.tool, ToolId::ClaudeCode | ToolId::Codex) {
        session_details(
            state.sessions.as_deref().unwrap_or_default(),
            SystemTime::now(),
        )
    } else {
        vec![]
    };
    let high = verge_core::domain::prioritized_sessions(
        state.sessions.as_deref().unwrap_or_default(),
        SystemTime::now(),
    )
    .iter()
    .filter(|s| {
        matches!(
            s.state,
            ActivityState::Working | ActivityState::WaitingOnUser
        ) && s
            .intelligence
            .as_ref()
            .and_then(|i| i.context.pressure(SystemTime::now()))
            .is_some_and(|p| p >= verge_core::domain::ContextPressure::High)
    })
    .count();
    let session_summary = (!sessions.is_empty()).then(|| {
        if high > 0 {
            format!("Sessions · {high} context high")
        } else {
            format!("Sessions · {}", sessions.len())
        }
    });
    ToolGlyph {
        sessions,
        session_summary,
        selected_session: None,
        label: label.to_string(),
        mark,
        brand_color,
        state: tint,
        permission: None,
        session_count: state.sessions.as_ref().map(|items| items.len() as u32),
        usage_windows: state
            .usage_windows
            .iter()
            .filter_map(|w| {
                let UsageReading::Fraction(f) = w.reading else {
                    return None;
                };
                let reset = w
                    .resets_at
                    .and_then(|at| at.duration_since(SystemTime::now()).ok())
                    .map(|left| {
                        let m = left.as_secs().div_ceil(60);
                        if m < 60 {
                            format!("Resets in {m} min")
                        } else if m < 1440 {
                            format!("Resets in {}h {}m", m / 60, m % 60)
                        } else {
                            format!("Resets in {}d {}h", m / 1440, (m % 1440) / 60)
                        }
                    })
                    .unwrap_or_else(|| "Reset unavailable".into());
                Some(verge_core::ports::UsageDetail {
                    label: w.name.clone(),
                    fraction: f.clamp(0.0, 1.0) as f32,
                    reset,
                    dimmed: matches!(w.recency, Recency::Aged { .. }),
                })
            })
            .collect(),
        reminder: state.reminder.clone(),
        activity_label,
        metric,
        dimmed,
        detail_lines,
    }
}

fn session_details(
    items: &[verge_core::domain::ActivitySession],
    now: SystemTime,
) -> Vec<verge_core::ports::SessionDetail> {
    use verge_core::domain::{
        prioritized_sessions, session_priority, ContextPressure, ContextUsage, Fidelity,
    };
    prioritized_sessions(items, now)
        .into_iter()
        .map(|s| {
            let intelligence = s.intelligence.as_ref();
            let model = intelligence
                .and_then(|i| i.model.as_ref())
                .map(|m| format!("Model · {}", m.name))
                .unwrap_or_else(|| "Model not reported".into());
            let context = intelligence.map(|i| &i.context);
            let context_line = match context {
                Some(ContextUsage::Observed {
                    fraction,
                    as_of,
                    fidelity,
                    ..
                }) if fraction.is_finite() && (0.0..=1.0).contains(fraction) && *as_of <= now => {
                    let stale = now.duration_since(*as_of).is_ok_and(|d| d.as_secs() > 120);
                    let prefix = if stale { "Last context" } else { "Context" };
                    let approximate = if *fidelity == Fidelity::Estimated {
                        "~"
                    } else {
                        ""
                    };
                    let pressure = if matches!(
                        s.state,
                        ActivityState::Working | ActivityState::WaitingOnUser
                    ) {
                        match context.unwrap().pressure(now) {
                            Some(ContextPressure::High) => " · High",
                            Some(ContextPressure::NearCapacity) => " · Near capacity",
                            _ => "",
                        }
                    } else {
                        ""
                    };
                    format!("{prefix} {approximate}{:.0}%{pressure}", fraction * 100.0)
                }
                Some(ContextUsage::Unavailable) => "Context unavailable".into(),
                _ => "Context not reported".into(),
            };
            let activity = match s.state {
                ActivityState::Working => "Working",
                ActivityState::WaitingOnUser => "Waiting for you",
                ActivityState::RecentlyIdle => "Idle",
                ActivityState::Completed => "Completed",
                ActivityState::Stopped => "Stopped / error",
                ActivityState::Disconnected => "Disconnected",
                ActivityState::Unknown => "Activity not reported",
            };
            let capacity = match context {
                Some(ContextUsage::Observed {
                    capacity: Some(n), ..
                }) => format!("Capacity · {n} tokens"),
                _ => "Capacity not reported".into(),
            };
            verge_core::ports::SessionDetail {
                id: s.id.clone(),
                title: s.name.clone(),
                lines: vec![model, context_line, activity.into(), capacity],
                priority: session_priority(s, now),
            }
        })
        .collect()
}

pub fn render_requested(states: &[AmbientState], open: bool) -> OverlayContent {
    let mut content = render(states);
    if open {
        let mut known = states
            .iter()
            .filter(|s| {
                matches!(
                    s.availability,
                    Availability::Available | Availability::Unsupported { .. }
                ) || s.sessions.as_ref().is_some_and(|v| !v.is_empty())
            })
            .map(render_one)
            .collect::<Vec<_>>();
        known.sort_by_key(|g| std::cmp::Reverse(tool_priority(g)));
        if !known.is_empty() {
            content.overflow_count =
                (known.len() > GLYPH_CAP).then_some(known.len().saturating_sub(GLYPH_CAP) as u32);
            content.glyphs = known.into_iter().take(GLYPH_CAP).collect();
        }
    }
    if open && content.glyphs.is_empty() {
        if let Some(state) = states.first() {
            content.glyphs.push(render_one(state));
        }
    }
    content
}

fn tool_priority(g: &ToolGlyph) -> u8 {
    let activity = match g.state {
        StateTint::Waiting => 5,
        StateTint::Stopped => 4,
        StateTint::Working => 2,
        _ => 0,
    };
    activity.max(g.sessions.iter().map(|s| s.priority).max().unwrap_or(0))
}

/// Platform rendering uses supplied Claude Code and Codex marks; other identities remain placeholders.
fn identity(tool: ToolId) -> (char, (u8, u8, u8), &'static str) {
    match tool {
        ToolId::ClaudeCode => ('✳', (217, 119, 87), "Claude"),
        ToolId::Cursor => ('▲', (232, 211, 62), "Cursor"),
        ToolId::Codex => ('◈', (235, 235, 235), "ChatGPT"),
        ToolId::Glm => ('◫', (77, 208, 225), "GLM"),
        ToolId::Antigravity => ('◎', (124, 155, 255), "Antigravity"),
        ToolId::Grok => ('✶', (185, 139, 255), "Grok"),
        ToolId::KiloCode => ('K', (235, 235, 235), "KiloCode"),
        ToolId::Hermes => ('H', (224, 189, 135), "Hermes"),
        ToolId::Pi => ('π', (235, 235, 235), "Pi"),
        ToolId::OpenCode => ('◇', (200, 200, 200), "OpenCode"),
    }
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
            usage_windows: vec![],
            reminder: None,
            sessions: None,
            activity: verge_core::domain::ActivityState::Unknown,
            account: account(ToolId::ClaudeCode),
            availability,
            most_constrained_window: window,
        }
    }

    #[test]
    fn session_projection_distinguishes_context_quota_and_stale_observations() {
        use verge_core::domain::{ActivitySession, ContextUsage, SessionIntelligence};
        let now = SystemTime::now();
        let mut sessions = vec![ActivitySession {
            id: "opaque-do-not-show".into(),
            name: "Example project".into(),
            state: ActivityState::Working,
            since: now,
            waiting_for: None,
            intelligence: Some(SessionIntelligence {
                model: None,
                context: ContextUsage::Observed {
                    fraction: 0.91,
                    capacity: Some(200000),
                    as_of: now,
                    fidelity: Fidelity::Official,
                },
            }),
        }];
        let details = session_details(&sessions, now);
        assert_eq!(details[0].priority, 3);
        assert!(details[0]
            .lines
            .contains(&"Context 91% · Near capacity".into()));
        assert!(details[0].lines.contains(&"Model not reported".into()));
        assert!(!details[0].lines.iter().any(|s| s.contains("opaque")));
        let stale = session_details(&sessions, now + std::time::Duration::from_secs(121));
        assert!(stale[0].lines.contains(&"Last context 91%".into()));
        assert_eq!(stale[0].priority, 2);
        if let ContextUsage::Observed { fidelity, .. } =
            &mut sessions[0].intelligence.as_mut().unwrap().context
        {
            *fidelity = Fidelity::Estimated;
        }
        assert!(session_details(&sessions, now)[0]
            .lines
            .contains(&"Context ~91%".into()));
        let mut state = state(Availability::NotMetered, None).with_sessions(Some(sessions));
        state.account.tool = ToolId::ClaudeCode;
        let g = render_one(&state);
        assert_eq!(
            g.metric,
            Metric::None,
            "context never becomes account quota"
        );
        assert_eq!(g.sessions.len(), 1);
        state.account.tool = ToolId::Codex;
        let codex = render_one(&state);
        assert_eq!(codex.label, "ChatGPT");
        assert_eq!(codex.sessions, g.sessions);
    }

    #[test]
    fn unauthenticated_never_shows_a_number_or_a_metric_ring() {
        let content = render(&[state(Availability::Unauthenticated, None)]);
        let glyph = &content.glyphs[0];
        assert_eq!(glyph.metric, Metric::None);
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
        assert_eq!(
            glyph.metric,
            Metric::Neutral,
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
        assert_eq!(content.glyphs[0].metric, Metric::Fraction(0.73));
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
    fn unknown_activity_remains_neutral() {
        let content = render(&[state(Availability::Available, None)]);
        assert_eq!(content.glyphs[0].state, StateTint::Neutral);
    }

    #[test]
    fn verified_activity_survives_missing_usage_and_prioritizes_attention() {
        use verge_core::domain::ActivitySession;
        let sessions = vec![ActivityState::Working, ActivityState::WaitingOnUser]
            .into_iter()
            .enumerate()
            .map(|(i, state)| ActivitySession {
                intelligence: None,
                id: i.to_string(),
                name: "Verge".into(),
                state,
                since: SystemTime::now(),
                waiting_for: Some("Review changes".into()),
            })
            .collect();
        let content =
            render(&[state(Availability::NotMetered, None).with_sessions(Some(sessions))]);
        let glyph = &content.glyphs[0];
        assert_eq!(glyph.session_count, Some(2));
        assert_eq!(glyph.state, StateTint::Waiting);
        assert_eq!(glyph.activity_label.as_deref(), Some("Needs you"));
        assert_eq!(glyph.metric, Metric::None);
        assert_eq!(&glyph.detail_lines[..2], &["Verge", "Review changes"]);
    }

    #[test]
    fn all_usage_windows_and_reminder_reach_presentation() {
        use verge_core::domain::{project_ambient_state, UsageSnapshot};
        let window = |name: &str, f| UsageWindow {
            name: name.into(),
            reading: UsageReading::Fraction(f),
            resets_at: Some(SystemTime::now() + std::time::Duration::from_secs(3600)),
            fidelity: Fidelity::Official,
            recency: Recency::Live {
                as_of: SystemTime::now(),
            },
        };
        let mut state = project_ambient_state(&UsageSnapshot {
            account: account(ToolId::ClaudeCode),
            availability: Availability::Available,
            windows: vec![window("Current session", 0.32), window("All models", 0.65)],
        });
        state.reminder = Some("Current session: 80% used".into());
        let content = render(&[state]);
        assert_eq!(content.glyphs[0].usage_windows.len(), 2);
        assert_eq!(content.glyphs[0].metric, Metric::Fraction(0.65));
        assert!(content.glyphs[0].reminder.is_some());
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
    fn empty_meter_is_idle_and_real_reset_is_formatted() {
        assert!(render(&[state(Availability::NotMetered, None)])
            .glyphs
            .is_empty());
        let window = UsageWindow {
            name: "Current window".into(),
            reading: UsageReading::Fraction(0.73),
            resets_at: Some(SystemTime::now() + std::time::Duration::from_secs(3059)),
            fidelity: Fidelity::Official,
            recency: Recency::Live {
                as_of: SystemTime::now(),
            },
        };
        let content = render(&[state(Availability::Available, Some(window))]);
        assert!(content.glyphs[0]
            .detail_lines
            .iter()
            .any(|line| line == "Resets in 51 min"));
    }

    #[test]
    fn no_overflow_when_within_cap() {
        let states: Vec<AmbientState> = (0..2)
            .map(|_| state(Availability::Unauthenticated, None))
            .collect();
        let content = render(&states);
        assert_eq!(content.overflow_count, None);
    }

    #[test]
    fn explicit_open_reveals_real_idle_data_without_inventing_usage() {
        let states = [state(Availability::NotMetered, None)];
        assert!(render_requested(&states, false).glyphs.is_empty());
        let opened = render_requested(&states, true);
        assert_eq!(opened.glyphs.len(), 1);
        assert_eq!(opened.glyphs[0].label, "Claude");
        assert_eq!(opened.glyphs[0].metric, Metric::None);
        assert!(render_requested(&[], true).glyphs.is_empty());
    }
}
