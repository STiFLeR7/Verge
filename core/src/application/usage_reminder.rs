use crate::domain::{Recency, UsageReading, UsageSnapshot};
use std::{collections::BTreeMap, time::SystemTime};

/// One reminder per threshold and reset window during this application run.
#[derive(Default)]
pub struct UsageReminders(BTreeMap<String, (Option<SystemTime>, u8)>);
impl UsageReminders {
    pub fn update(&mut self, snapshot: &UsageSnapshot, now: SystemTime) -> Option<String> {
        let mut message = None;
        for w in &snapshot.windows {
            let UsageReading::Fraction(f) = w.reading else {
                continue;
            };
            if matches!(w.recency, Recency::Aged { .. }) || w.resets_at.is_some_and(|at| at <= now)
            {
                continue;
            }
            let band = if f >= 1.0 {
                100
            } else if f >= 0.95 {
                95
            } else if f >= 0.8 {
                80
            } else {
                0
            };
            let key = format!(
                "{:?}:{}:{}",
                snapshot.account.tool, snapshot.account.label, w.name
            );
            let previous = self.0.entry(key).or_insert((w.resets_at, 0));
            if previous.0 != w.resets_at {
                *previous = (w.resets_at, 0);
            }
            if band > previous.1 {
                message = Some(if band == 100 {
                    format!("{} limit reached", w.name)
                } else {
                    format!("{}: {:.0}% used", w.name, f * 100.0)
                });
                previous.1 = band;
            }
        }
        message
    }
}

#[cfg(test)]
#[test]
fn reminds_once_and_rearms_on_a_new_window() {
    use crate::domain::*;
    let now = SystemTime::now();
    let mut s = UsageSnapshot {
        account: Account {
            tool: ToolId::ClaudeCode,
            label: "Claude".into(),
            provenance: "test".into(),
        },
        availability: Availability::Available,
        windows: vec![UsageWindow {
            name: "5h".into(),
            reading: UsageReading::Fraction(0.81),
            resets_at: Some(now + std::time::Duration::from_secs(100)),
            fidelity: Fidelity::Official,
            recency: Recency::Live { as_of: now },
        }],
    };
    let mut reminders = UsageReminders::default();
    assert!(reminders.update(&s, now).is_some());
    assert!(reminders.update(&s, now).is_none());
    s.windows[0].reading = UsageReading::Fraction(0.96);
    assert!(reminders.update(&s, now).is_some());
    s.windows[0].recency = Recency::Aged { as_of: now };
    s.windows[0].reading = UsageReading::Fraction(1.0);
    assert!(reminders.update(&s, now).is_none());
    s.windows[0].recency = Recency::Live { as_of: now };
    s.windows[0].resets_at = Some(now + std::time::Duration::from_secs(200));
    assert!(reminders.update(&s, now).is_some());
}
