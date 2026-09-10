use super::{ActivitySession, ActivityState, Fidelity, Recency};
use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ModelIdentity {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ContextUsage {
    #[default]
    Unknown,
    Unavailable,
    Observed {
        fraction: f64,
        capacity: Option<u64>,
        as_of: SystemTime,
        fidelity: Fidelity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextPressure {
    Normal,
    High,
    NearCapacity,
}

impl ContextUsage {
    pub fn pressure(&self, now: SystemTime) -> Option<ContextPressure> {
        let Self::Observed {
            fraction,
            capacity,
            as_of,
            fidelity,
        } = self
        else {
            return None;
        };
        if !fraction.is_finite()
            || !(0.0..=1.0).contains(fraction)
            || *capacity == Some(0)
            || *as_of > now
            || *fidelity == Fidelity::Estimated
            || matches!(
                Recency::classify(*as_of, now, Duration::from_secs(120)),
                Recency::Aged { .. }
            )
        {
            return None;
        }
        Some(if *fraction >= 0.9 {
            ContextPressure::NearCapacity
        } else if *fraction >= 0.8 {
            ContextPressure::High
        } else {
            ContextPressure::Normal
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionIntelligence {
    pub model: Option<ModelIdentity>,
    pub context: ContextUsage,
}

pub fn session_priority(session: &ActivitySession, now: SystemTime) -> u8 {
    match session.state {
        ActivityState::WaitingOnUser => 5,
        ActivityState::Stopped
            if now
                .duration_since(session.since)
                .is_ok_and(|d| d.as_secs() < super::COMPLETED_GRACE_SECONDS) =>
        {
            4
        }
        ActivityState::Working => {
            if session
                .intelligence
                .as_ref()
                .and_then(|i| i.context.pressure(now))
                .is_some_and(|p| p >= ContextPressure::High)
            {
                3
            } else {
                2
            }
        }
        ActivityState::Unknown => 1,
        _ => 0,
    }
}

/// IDs are scoped to the enclosing account/tool. Never deduplicate across accounts.
pub fn prioritized_sessions(
    sessions: &[ActivitySession],
    now: SystemTime,
) -> Vec<&ActivitySession> {
    let mut unique = BTreeMap::<&str, &ActivitySession>::new();
    for session in sessions {
        if unique
            .get(session.id.as_str())
            .is_none_or(|old| session.since > old.since)
        {
            unique.insert(&session.id, session);
        }
    }
    let mut result: Vec<_> = unique.into_values().collect();
    let fullness = |s: &ActivitySession| {
        s.intelligence
            .as_ref()
            .and_then(|i| {
                if i.context.pressure(now).is_some() {
                    if let ContextUsage::Observed { fraction, .. } = i.context {
                        return Some((fraction * 10000.0) as u32);
                    }
                }
                None
            })
            .unwrap_or(0)
    };
    result.sort_by_key(|s| {
        (
            std::cmp::Reverse(session_priority(s, now)),
            std::cmp::Reverse(fullness(s)),
        )
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_and_attention_preserve_uncertainty_and_session_identity() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let session = |id: &str, fraction| ActivitySession {
            id: id.into(),
            name: "Project".into(),
            state: ActivityState::Working,
            since: now,
            waiting_for: None,
            intelligence: Some(SessionIntelligence {
                model: None,
                context: ContextUsage::Observed {
                    fraction,
                    capacity: Some(200000),
                    as_of: now,
                    fidelity: Fidelity::Official,
                },
            }),
        };
        let mut items = vec![session("one", 0.2)];
        assert_eq!(prioritized_sessions(&items, now).len(), 1);
        items.extend([
            session("two", 0.82),
            session("three", 0.91),
            session("one", 0.1),
        ]);
        assert_eq!(prioritized_sessions(&items, now).len(), 3);
        assert_eq!(prioritized_sessions(&items, now)[0].id, "three");
        assert_eq!(
            items
                .iter()
                .filter(|s| session_priority(s, now) == 3)
                .count(),
            2
        );
        items[0].state = ActivityState::WaitingOnUser;
        assert_eq!(prioritized_sessions(&items, now)[0].id, "one");
        for state in [ActivityState::Completed, ActivityState::Disconnected] {
            items[2].state = state;
            assert_eq!(session_priority(&items[2], now), 0);
        }
        for context in [
            ContextUsage::Unknown,
            ContextUsage::Unavailable,
            ContextUsage::Observed {
                fraction: 0.95,
                capacity: None,
                as_of: now - Duration::from_secs(121),
                fidelity: Fidelity::Official,
            },
            ContextUsage::Observed {
                fraction: 0.95,
                capacity: None,
                as_of: now,
                fidelity: Fidelity::Estimated,
            },
            ContextUsage::Observed {
                fraction: f64::NAN,
                capacity: None,
                as_of: now,
                fidelity: Fidelity::Official,
            },
        ] {
            assert_eq!(context.pressure(now), None);
        }
        // Same local ID in a different account is projected independently.
        assert_eq!(
            prioritized_sessions(&[session("one", 0.99)], now)[0]
                .intelligence
                .as_ref()
                .unwrap()
                .context
                .pressure(now),
            Some(ContextPressure::NearCapacity)
        );
        assert!(items[0].intelligence.as_ref().unwrap().model.is_none());
    }
}
