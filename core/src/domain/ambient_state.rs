use super::{Account, Availability, UsageSnapshot, UsageWindow};

/// What the ambient surface should actually show for this account, right
/// now. A pure projection, not a stored entity — generalizes the reference
/// product's "most-constrained window wins" rule into an explicit function
/// instead of implicit UI logic.
///
/// Activity fusion (folding in `ActivitySession`) is not wired in yet: the
/// first vertical slice only has a `UsageSource` for Claude Code. Adding an
/// `ActivitySource` is the next capability to compose here, not a redesign
/// of this function's shape.
#[derive(Debug, Clone, PartialEq)]
pub struct AmbientState {
    pub account: Account,
    pub availability: Availability,
    pub most_constrained_window: Option<UsageWindow>,
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

    fn window(name: &str, reading: super::super::UsageReading) -> UsageWindow {
        UsageWindow {
            name: name.into(),
            reading,
            resets_at: None,
            fidelity: Fidelity::Derived,
            recency: Recency::Live { as_of: SystemTime::now() },
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
        assert!(project_ambient_state(&snapshot).most_constrained_window.is_none());
    }
}
