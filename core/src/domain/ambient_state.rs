use super::{Account, ActivitySession, ActivityState, Availability, UsageSnapshot, UsageWindow};

/// What the ambient surface should actually show for this account, right
/// now. A pure projection, not a stored entity â€” generalizes the reference
/// product's "most-constrained window wins" rule into an explicit function
/// instead of implicit UI logic.
///
/// Usage and verified live sessions are composed independently.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientState {
    pub account: Account,
    pub availability: Availability,
    pub most_constrained_window: Option<UsageWindow>,
    pub usage_windows: Vec<UsageWindow>,
    pub reminder: Option<String>,
    /// None means activity could not be read; Some(empty) means no live sessions.
    pub sessions: Option<Vec<ActivitySession>>,
    pub activity: ActivityState,
}

/// Picks the most-constrained window, by `UsageReading`, out of a snapshot.
/// A `Fraction` window is more constrained the higher its value; a `Count`
/// window has no ceiling to compare against and is only ever picked when no
/// `Fraction` window exists, favoring the highest count.
///
/// Never invents a window: if `availability` is not `Available`,
/// `most_constrained_window` is always `None`, regardless of what
/// `windows` contains.
pub fn project_ambient_state(snapshot: &UsageSnapshot) -> AmbientState {
    let most_constrained_window = if snapshot.availability == Availability::Available {
        most_constrained(&snapshot.windows)
    } else {
        None
    };

    AmbientState {
        account: snapshot.account.clone(),
        availability: snapshot.availability.clone(),
        most_constrained_window,
        usage_windows: if snapshot.availability == Availability::Available {
            snapshot.windows.clone()
        } else {
            vec![]
        },
        reminder: None,
        sessions: None,
        activity: ActivityState::Unknown,
    }
}

impl AmbientState {
    /// Attention outranks work. Usage availability never suppresses known activity.
    pub fn with_sessions(mut self, sessions: Option<Vec<ActivitySession>>) -> Self {
        self.activity = sessions
            .as_ref()
            .and_then(|items| {
                items.iter().max_by_key(|s| match s.state {
                    ActivityState::WaitingOnUser => 5,
                    ActivityState::Stopped => {
                        if s.since
                            .elapsed()
                            .is_ok_and(|age| age.as_secs() < super::COMPLETED_GRACE_SECONDS)
                        {
                            4
                        } else {
                            0
                        }
                    }
                    ActivityState::Working => 3,
                    ActivityState::Completed => {
                        if s.since
                            .elapsed()
                            .is_ok_and(|age| age.as_secs() < super::COMPLETED_GRACE_SECONDS)
                        {
                            2
                        } else {
                            0
                        }
                    }
                    ActivityState::RecentlyIdle => 1,
                    ActivityState::Unknown | ActivityState::Disconnected => 0,
                })
            })
            .map_or(ActivityState::Unknown, |s| s.state);
        self.sessions = sessions;
        self
    }
}

fn most_constrained(windows: &[UsageWindow]) -> Option<UsageWindow> {
    use super::UsageReading::*;

    windows
        .iter()
        .max_by(|a, b| {
            use std::cmp::Ordering;
            match (a.reading, b.reading) {
                (Fraction(x), Fraction(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                (Fraction(_), Count(_)) => Ordering::Greater,
                (Count(_), Fraction(_)) => Ordering::Less,
                (Count(x), Count(y)) => x.cmp(&y),
            }
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Fidelity, Recency, ToolId};
    use std::time::SystemTime;

    fn account() -> Account {
        Account {
            tool: ToolId::ClaudeCode,
            label: "test".into(),
            provenance: "test fixture".into(),
        }
    }

    /// A second tool's identity, used to prove this projection is generic
    /// over `ToolId` rather than coincidentally correct for one â€” see
    /// `docs/design/SECOND_VERTICAL_SLICE.md`.
    fn codex_account() -> Account {
        Account {
            tool: ToolId::Codex,
            label: "Codex".into(),
            provenance: "test fixture".into(),
        }
    }

    fn window(name: &str, reading: super::super::UsageReading) -> UsageWindow {
        UsageWindow {
            name: name.into(),
            reading,
            resets_at: None,
            fidelity: Fidelity::Derived,
            recency: Recency::Live {
                as_of: SystemTime::now(),
            },
        }
    }

    #[test]
    fn unavailable_snapshot_never_yields_a_window() {
        let snapshot = UsageSnapshot {
            account: account(),
            availability: Availability::Unauthenticated,
            windows: vec![window("5h", super::super::UsageReading::Fraction(0.99))],
        };
        let state = project_ambient_state(&snapshot);
        assert!(state.most_constrained_window.is_none());
    }

    /// `Unsupported` is the outcome Codex specifically needs (no local
    /// usage cache exists â€” see docs/design/codex-linux-local-state.md).
    /// Proves the same "never invent" chokepoint applies regardless of
    /// which tool's `Account` is attached, and regardless of *why*
    /// availability isn't `Available`, not just for `Unauthenticated`.
    #[test]
    fn unsupported_availability_never_yields_a_window_for_a_second_tool() {
        let snapshot = UsageSnapshot {
            account: codex_account(),
            availability: Availability::Unsupported {
                reason: "no local usage cache".into(),
            },
            windows: vec![],
        };
        let state = project_ambient_state(&snapshot);
        assert!(state.most_constrained_window.is_none());
        assert_eq!(state.account.tool, ToolId::Codex);
    }

    #[test]
    fn picks_highest_fraction() {
        let snapshot = UsageSnapshot {
            account: account(),
            availability: Availability::Available,
            windows: vec![
                window("5h", super::super::UsageReading::Fraction(0.4)),
                window("weekly", super::super::UsageReading::Fraction(0.9)),
            ],
        };
        let state = project_ambient_state(&snapshot);
        assert_eq!(state.most_constrained_window.unwrap().name, "weekly");
    }

    #[test]
    fn fraction_beats_bare_count_regardless_of_magnitude() {
        let snapshot = UsageSnapshot {
            account: account(),
            availability: Availability::Available,
            windows: vec![
                window("today's messages", super::super::UsageReading::Count(9_000)),
                window("5h", super::super::UsageReading::Fraction(0.1)),
            ],
        };
        let state = project_ambient_state(&snapshot);
        assert_eq!(state.most_constrained_window.unwrap().name, "5h");
    }

    #[test]
    fn no_windows_yields_none_even_when_available() {
        let snapshot = UsageSnapshot {
            account: account(),
            availability: Availability::Available,
            windows: vec![],
        };
        assert!(project_ambient_state(&snapshot)
            .most_constrained_window
            .is_none());
    }
}
