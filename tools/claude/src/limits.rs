use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use verge_core::domain::{
    Account, Availability, Fidelity, Recency, UsageReading, UsageSnapshot, UsageWindow,
};

pub fn default_signal_directory() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(|p| PathBuf::from(p).join("Verge").join("signals"))
}

/// Rate limits supplied by Claude's documented status-line input, not inferred from tokens.
pub fn read_limits(account: &Account) -> Option<UsageSnapshot> {
    let path = default_signal_directory()?.join("claude-usage.json");
    if fs::metadata(&path).ok()?.len() > 65536 {
        return None;
    }
    parse_limits(account, &fs::read(path).ok()?, SystemTime::now())
}

fn parse_limits(account: &Account, raw: &[u8], now: SystemTime) -> Option<UsageSnapshot> {
    let data: serde_json::Value = serde_json::from_slice(raw).ok()?;
    let at = SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(data["at"].as_u64()?))?;
    if at > now + Duration::from_secs(30) {
        return None;
    }
    let windows = [
        ("five_hour", "Current session"),
        ("seven_day", "All models"),
    ]
    .into_iter()
    .filter_map(|(key, name)| {
        let w = &data["rate_limits"][key];
        let percent = w["used_percentage"].as_f64()?;
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            return None;
        }
        let reset =
            SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(w["resets_at"].as_u64()?))?;
        if reset <= now {
            return None;
        }
        Some(UsageWindow {
            name: name.into(),
            reading: UsageReading::Fraction(percent / 100.0),
            resets_at: Some(reset),
            fidelity: Fidelity::Official,
            recency: Recency::classify(at, now, Duration::from_secs(900)),
        })
    })
    .collect::<Vec<_>>();
    (!windows.is_empty()).then(|| UsageSnapshot {
        account: account.clone(),
        availability: Availability::Available,
        windows,
    })
}

#[cfg(test)]
#[test]
fn limits_preserve_both_windows_and_reject_expired_or_invalid_data() {
    use verge_core::domain::ToolId;
    let account = Account {
        tool: ToolId::ClaudeCode,
        label: "Claude".into(),
        provenance: "test".into(),
    };
    let raw=br#"{"at":100000,"rate_limits":{"five_hour":{"used_percentage":73,"resets_at":200},"seven_day":{"used_percentage":7,"resets_at":300}}}"#;
    let snapshot = parse_limits(
        &account,
        raw,
        SystemTime::UNIX_EPOCH + Duration::from_secs(101),
    )
    .unwrap();
    assert_eq!(snapshot.windows.len(), 2);
    assert_eq!(snapshot.windows[0].reading, UsageReading::Fraction(0.73));
    assert!(parse_limits(
        &account,
        raw,
        SystemTime::UNIX_EPOCH + Duration::from_secs(301)
    )
    .is_none());
    assert!(parse_limits(&account, b"{}", SystemTime::now()).is_none());
}
