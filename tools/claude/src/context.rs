use serde_json::Value;
use std::time::{Duration, SystemTime};
use verge_core::domain::{ContextUsage, Fidelity, ModelIdentity, SessionIntelligence};

/// Whitelisted status-line metadata only. A wrong identity or timestamp is not a new observation.
pub(crate) fn parse(
    raw: &[u8],
    id: &str,
    started: SystemTime,
    now: SystemTime,
) -> Option<SessionIntelligence> {
    let value: Value = serde_json::from_slice(raw).ok()?;
    if value["session_id"].as_str()? != id {
        return None;
    }
    let at = SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(value["at"].as_u64()?))?;
    if at < started || at > now {
        return None;
    }
    let clean = |s: &str| {
        s.chars()
            .filter(|c| {
                !c.is_control() && !matches!(*c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
            .take(160)
            .collect::<String>()
    };
    let model = value["model"]["id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|id| ModelIdentity {
            id: clean(id),
            name: clean(
                value["model"]["name"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(id),
            ),
        });
    let context = match value.get("context") {
        None => ContextUsage::Unknown,
        Some(v) if v.is_null() => ContextUsage::Unavailable,
        Some(v) => {
            let pct = v["used_percentage"].as_f64()?;
            if !pct.is_finite() || !(0.0..=100.0).contains(&pct) {
                return None;
            }
            let capacity = v["capacity"].as_u64();
            if capacity == Some(0) {
                return None;
            }
            ContextUsage::Observed {
                fraction: pct / 100.0,
                capacity,
                as_of: at,
                fidelity: Fidelity::Official,
            }
        }
    };
    Some(SessionIntelligence { model, context })
}

#[test]
fn statusline_contract_rejects_wrong_sessions_and_preserves_missing_data() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let start = now - Duration::from_secs(10);
    let valid = br#"{"session_id":"one","at":100000,"model":{"id":"model-one","name":"Model One"},"context":{"used_percentage":82,"capacity":200000}}"#;
    let data = parse(valid, "one", start, now).unwrap();
    assert_eq!(data.model.unwrap().name, "Model One");
    assert_eq!(
        data.context.pressure(now),
        Some(verge_core::domain::ContextPressure::High)
    );
    assert!(parse(valid, "two", start, now).is_none());
    assert!(parse(valid, "one", start, start).is_none());
    assert!(parse(b"{", "one", start, now).is_none());
    for (field, expected) in [
        ("", ContextUsage::Unknown),
        (",\"context\":null", ContextUsage::Unavailable),
    ] {
        let raw = format!("{{\"session_id\":\"one\",\"at\":100000{field}}}");
        let parsed = parse(raw.as_bytes(), "one", start, now).unwrap();
        assert_eq!(parsed.context, expected);
        assert!(parsed.model.is_none());
    }
    let bad = String::from_utf8_lossy(valid).replace("82", "182");
    assert!(parse(bad.as_bytes(), "one", start, now).is_none());
}
