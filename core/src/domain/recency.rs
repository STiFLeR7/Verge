use std::time::SystemTime;

/// How current a reading is, independent of how it was derived (`Fidelity`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recency {
    Live { as_of: SystemTime },
    Recent { as_of: SystemTime },
    Aged { as_of: SystemTime },
}

impl Recency {
    pub fn as_of(&self) -> SystemTime {
        match self {
            Recency::Live { as_of } | Recency::Recent { as_of } | Recency::Aged { as_of } => {
                *as_of
            }
        }
    }

    /// Classify an `as_of` timestamp against the caller-supplied trust
    /// window boundaries. Pure function, deterministic, no wall-clock
    /// dependency beyond the two instants given.
    pub fn classify(as_of: SystemTime, now: SystemTime, recent_within: std::time::Duration) -> Self {
        match now.duration_since(as_of) {
            Ok(age) if age.is_zero() => Recency::Live { as_of },
            Ok(age) if age <= recent_within => Recency::Recent { as_of },
            Ok(_) => Recency::Aged { as_of },
            // Clock skew (as_of is in the future from `now`'s perspective):
            // treat as live rather than invent a negative age.
            Err(_) => Recency::Live { as_of },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn zero_age_is_live() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        assert_eq!(Recency::classify(t, t, Duration::from_secs(60)), Recency::Live { as_of: t });
    }

    #[test]
    fn within_window_is_recent() {
        let as_of = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let now = as_of + Duration::from_secs(30);
        assert_eq!(
            Recency::classify(as_of, now, Duration::from_secs(60)),
            Recency::Recent { as_of }
        );
    }

    #[test]
    fn past_window_is_aged() {
        let as_of = SystemTime::UNIX_EPOCH + Duration::from_secs(1000);
        let now = as_of + Duration::from_secs(61);
        assert_eq!(
            Recency::classify(as_of, now, Duration::from_secs(60)),
            Recency::Aged { as_of }
        );
    }
}
