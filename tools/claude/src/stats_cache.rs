use serde::Deserialize;

/// Mirrors the subset of `~/.claude/stats-cache.json` this adapter reads.
/// `#[serde(default)]` throughout because this file is Claude Code's own
/// internal cache, not a published schema this adapter owns — a future
/// version adding or renaming fields must not turn a successful read into a
/// parse `Error`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsCache {
    #[serde(default)]
    pub daily_activity: Vec<DailyActivity>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DailyActivity {
    /// `YYYY-MM-DD`, e.g. "2026-08-20". Not parsed into a date type here —
    /// this adapter only needs the entries in file order, not calendar math.
    pub date: String,
    #[serde(default)]
    pub message_count: u64,
}

impl StatsCache {
    /// The most recent day this cache actually has data for — deliberately
    /// *not* "today's count", because `lastComputedDate` can be well behind
    /// the current date (observed directly: a cache last computed weeks
    /// before the file's own mtime). Claiming "today" when the freshest
    /// entry is three weeks old would be exactly the kind of silent
    /// misrepresentation the architecture's fidelity/recency split exists
    /// to prevent.
    pub fn most_recent_day_message_count(&self) -> Option<(&str, u64)> {
        self.daily_activity
            .last()
            .map(|entry| (entry.date.as_str(), entry.message_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_shape_and_picks_last_entry() {
        let json = r#"{
            "dailyActivity": [
                {"date": "2026-03-04", "messageCount": 540, "sessionCount": 7, "toolCallCount": 56},
                {"date": "2026-03-05", "messageCount": 2432, "sessionCount": 14, "toolCallCount": 350}
            ]
        }"#;
        let cache: StatsCache = serde_json::from_str(json).unwrap();
        assert_eq!(cache.most_recent_day_message_count(), Some(("2026-03-05", 2432)));
    }

    #[test]
    fn empty_activity_yields_none_not_zero() {
        let cache: StatsCache = serde_json::from_str(r#"{"dailyActivity": []}"#).unwrap();
        assert_eq!(cache.most_recent_day_message_count(), None);
    }

    #[test]
    fn missing_field_does_not_fail_to_parse() {
        let cache: StatsCache = serde_json::from_str(r#"{"totalSessions": 5}"#).unwrap();
        assert_eq!(cache.most_recent_day_message_count(), None);
    }
}
