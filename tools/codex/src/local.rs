//! Read only Codex rollout metadata and reported rate limits. No prompts or tool output are exposed.
use serde_json::Value;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use verge_core::domain::*;

pub fn default_sessions_path() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(|p| PathBuf::from(p).join(".codex"))
        })
        .map(|p| p.join("sessions"))
}

/// Bounded tail reads keep large conversations off the hot path. Older records stay unavailable.
pub fn read_state(directory: &Path) -> AmbientState {
    let now = SystemTime::now();
    let account = Account {
        tool: ToolId::Codex,
        label: "Codex".into(),
        provenance: "Codex local rollout metadata".into(),
    };
    let mut files = Vec::new();
    collect(directory, 3, &mut files);
    files.sort_by_key(|(_, at)| std::cmp::Reverse(*at));
    // ponytail: latest 32 rollout tails; increase this cap only for larger concurrent workloads.
    let mut sessions = Vec::new();
    let mut newest = None;
    for (path, modified) in files.into_iter().take(32) {
        if now.duration_since(modified).unwrap_or_default() > Duration::from_secs(86400) {
            continue;
        }
        let Ok(mut file) = fs::File::open(&path) else {
            continue;
        };
        let Some((activity, limits, metadata)) = read_rollout(&mut file, now) else {
            continue;
        };
        let activity = activity.or(metadata.observed_at.map(|at| (ActivityState::Unknown, at)));
        if let Some((at, windows)) = limits {
            if newest.as_ref().is_none_or(|(previous, _)| at > *previous) {
                newest = Some((at, windows));
            }
        }
        if let Some((state, since)) = activity {
            if now.duration_since(since).unwrap_or_default() < Duration::from_secs(600) {
                sessions.push(ActivitySession {
                    intelligence: Some(metadata.intelligence),
                    id: path.to_string_lossy().into_owned(),
                    name: metadata.name.unwrap_or_else(|| "Codex task".into()),
                    state,
                    waiting_for: None,
                    since,
                });
            }
        }
    }
    let windows = newest.map(|(_, windows)| windows).unwrap_or_default();
    project_ambient_state(&UsageSnapshot {
        account,
        availability: if windows.is_empty() {
            Availability::NotMetered
        } else {
            Availability::Available
        },
        windows,
    })
    .with_sessions(Some(sessions))
}

fn read_rollout(file: &mut fs::File, now: SystemTime) -> Option<TailState> {
    let length = file.metadata().ok()?.len();
    let mut budget = 512 * 1024;
    loop {
        let offset = length.saturating_sub(budget);
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut raw = Vec::new();
        (&mut *file).take(budget).read_to_end(&mut raw).ok()?;
        let parsed = parse_tail(&raw, now);
        // ponytail: bounded metadata backfill; keep Unknown beyond 4 MiB rather than scan transcripts.
        if parsed.2.saw_turn || offset == 0 || budget == 4 * 1024 * 1024 {
            return Some(parsed);
        }
        budget *= 2;
    }
}

fn collect(dir: &Path, depth: u8, files: &mut Vec<(PathBuf, SystemTime)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() && depth > 0 {
            collect(&entry.path(), depth - 1, files);
        } else if kind.is_file() && entry.path().extension().is_some_and(|e| e == "jsonl") {
            if let Ok(at) = entry.metadata().and_then(|m| m.modified()) {
                files.push((entry.path(), at));
            }
        }
    }
}

#[derive(Default)]
struct SessionMetadata {
    saw_turn: bool,
    observed_at: Option<SystemTime>,
    name: Option<String>,
    intelligence: SessionIntelligence,
}

// Only a human-readable basename/model crosses into presentation.
fn label(value: &Value) -> Option<String> {
    let text: String = value
        .as_str()?
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .take(160)
        .collect();
    (!text.trim().is_empty()).then(|| text.trim().to_owned())
}

type TailState = (
    Option<(ActivityState, SystemTime)>,
    Option<(SystemTime, Vec<UsageWindow>)>,
    SessionMetadata,
);
fn parse_tail(raw: &[u8], now: SystemTime) -> TailState {
    let mut activity = None;
    let mut limits = None;
    let mut metadata = SessionMetadata::default();
    let mut latest = SystemTime::UNIX_EPOCH;
    for line in raw.split(|b| *b == b'\n') {
        let Ok(v) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        let Some(at) = v["timestamp"].as_str().and_then(timestamp) else {
            continue;
        };
        if at > now || at < latest {
            continue;
        }
        latest = at;
        let p = &v["payload"];
        if v["type"] == "turn_context" {
            metadata.saw_turn = true;
            metadata.observed_at = Some(at);
            let model = label(&p["model"]).map(|name| ModelIdentity {
                id: name.clone(),
                name,
            });
            if model != metadata.intelligence.model {
                metadata.intelligence.context = ContextUsage::Unknown;
            }
            metadata.intelligence.model = model;
            metadata.name = p["cwd"]
                .as_str()
                .and_then(|cwd| cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next())
                .and_then(|name| label(&Value::String(name.into())));
            continue;
        }
        if v["type"] == "compacted" {
            metadata.intelligence.context = ContextUsage::Unknown;
            continue;
        }
        if v["type"] != "event_msg" {
            continue;
        }
        let state = match p["type"].as_str() {
            Some("task_started" | "item_completed") => Some(ActivityState::Working),
            Some("task_complete") => Some(ActivityState::Completed),
            Some("turn_aborted" | "error") => Some(ActivityState::Stopped),
            _ => None,
        };
        if let Some(state) = state {
            activity = Some((state, at));
        }
        if p["type"] == "token_count" {
            metadata.observed_at = Some(at);
            if p.get("info").is_some() {
                let info = &p["info"];
                metadata.intelligence.context = match (
                    info["last_token_usage"]["total_tokens"].as_u64(),
                    info["model_context_window"].as_u64(),
                ) {
                    (Some(used), Some(capacity)) if capacity > 0 && used <= capacity => {
                        ContextUsage::Observed {
                            fraction: used as f64 / capacity as f64,
                            capacity: Some(capacity),
                            as_of: at,
                            fidelity: Fidelity::Derived,
                        }
                    }
                    _ => ContextUsage::Unavailable,
                };
            }
            // Token reports advance known working activity, never resurrect a completed turn.
            if let Some((ActivityState::Working, since)) = activity.as_mut() {
                *since = at;
            }
            let rates = &p["rate_limits"];
            if rates["limit_id"].as_str().is_some_and(|id| id != "codex") {
                continue;
            }
            let windows = ["primary", "secondary"]
                .into_iter()
                .filter_map(|key| {
                    let w = &rates[key];
                    let percent = w["used_percent"].as_f64()?;
                    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
                        return None;
                    }
                    let reset = SystemTime::UNIX_EPOCH
                        .checked_add(Duration::from_secs(w["resets_at"].as_u64()?))?;
                    if reset <= now {
                        return None;
                    }
                    let minutes = w["window_minutes"].as_u64()?;
                    Some(UsageWindow {
                        name: if minutes == 10080 {
                            "Codex weekly".into()
                        } else if minutes == 300 {
                            "Codex 5-hour".into()
                        } else {
                            "Codex limit".into()
                        },
                        reading: UsageReading::Fraction(percent / 100.0),
                        resets_at: Some(reset),
                        fidelity: Fidelity::Official,
                        recency: Recency::classify(at, now, Duration::from_secs(900)),
                    })
                })
                .collect::<Vec<_>>();
            if !windows.is_empty() {
                limits = Some((at, windows));
            }
        }
    }
    // A crashed or silent session is not indefinitely reported as working.
    if let Some((state, at)) = activity.as_mut() {
        if *state == ActivityState::Working
            && now.duration_since(*at).unwrap_or_default() > Duration::from_secs(120)
        {
            *state = ActivityState::Unknown;
        }
    }
    (activity, limits, metadata)
}

// Rollout timestamps use UTC RFC3339. Reject other representations instead of guessing.
fn timestamp(s: &str) -> Option<SystemTime> {
    if !s.is_ascii()
        || s.len() < 20
        || !s.ends_with('Z')
        || s.get(4..5) != Some("-")
        || s.get(7..8) != Some("-")
        || s.get(10..11) != Some("T")
        || s.get(13..14) != Some(":")
        || s.get(16..17) != Some(":")
    {
        return None;
    }
    let n = |a, b| s.get(a..b)?.parse::<u64>().ok();
    let (year, month, day, hour, minute, second) = (
        n(0, 4)?,
        n(5, 7)?,
        n(8, 10)?,
        n(11, 13)?,
        n(14, 16)?,
        n(17, 19)?,
    );
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let leap = |y: u64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day == 0 || day > months[month as usize - 1] {
        return None;
    }
    let days: u64 = (1970..year)
        .map(|y| if leap(y) { 366 } else { 365 })
        .sum::<u64>()
        + months[..month as usize - 1].iter().sum::<u64>()
        + day
        - 1;
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(
        days * 86400 + hour * 3600 + minute * 60 + second,
    ))
}

#[test]
fn reported_limits_and_activity_never_invent_usage_or_persist_work_forever() {
    let now = timestamp("2026-09-08T09:00:00Z").unwrap();
    let reset = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1000;
    let raw = format!(
        r#"{{"timestamp":"2026-09-08T08:59:59Z","type":"event_msg","payload":{{"type":"task_started"}}}}
{{"timestamp":"2026-09-08T08:59:59Z","type":"event_msg","payload":{{"type":"token_count","rate_limits":{{"limit_id":"codex","primary":{{"used_percent":29,"window_minutes":10080,"resets_at":{reset}}}}}}}}}"#
    );
    let (activity, limits, _) = parse_tail(raw.as_bytes(), now);
    assert_eq!(activity.unwrap().0, ActivityState::Working);
    assert_eq!(limits.unwrap().1[0].reading, UsageReading::Fraction(0.29));
    assert_eq!(
        parse_tail(raw.as_bytes(), now + Duration::from_secs(180))
            .0
            .unwrap()
            .0,
        ActivityState::Unknown
    );
    assert!(parse_tail(raw.as_bytes(), now + Duration::from_secs(1001))
        .1
        .is_none());
    assert!(parse_tail(raw.replace("29,", "129,").as_bytes(), now)
        .1
        .is_none());
    assert!(parse_tail(b"partial", now).1.is_none());
    assert!(timestamp("2026-02-30T09:00:00Z").is_none());
    assert_eq!(
        timestamp("1970-01-01T00:00:00Z"),
        Some(SystemTime::UNIX_EPOCH)
    );
}

#[test]
fn session_metadata_tracks_model_context_and_resets_without_using_cumulative_usage() {
    let now = timestamp("2026-09-08T09:00:00Z").unwrap();
    let event = |kind: &str, payload: Value| {
        serde_json::json!({
            "timestamp": "2026-09-08T08:59:59Z", "type": kind, "payload": payload
        })
        .to_string()
    };
    let turn = event(
        "turn_context",
        serde_json::json!({"model":"example-model", "cwd":"D:\\Example", "developer_instructions":"NEVER EXPOSE"}),
    );
    let count = event(
        "event_msg",
        serde_json::json!({"type":"token_count", "info": {
            "last_token_usage":{"total_tokens":82000}, "total_token_usage":{"total_tokens":9000000}, "model_context_window":100000
        }}),
    );
    let raw = format!("{turn}\n{count}\npartial");
    let (_, _, m) = parse_tail(raw.as_bytes(), now);
    assert_eq!(m.name.as_deref(), Some("Example"));
    assert_eq!(m.intelligence.model.unwrap().name, "example-model");
    assert!(matches!(
        m.intelligence.context,
        ContextUsage::Observed {
            fraction: 0.82,
            capacity: Some(100000),
            fidelity: Fidelity::Derived,
            ..
        }
    ));
    assert_eq!(
        m.intelligence.context.pressure(now),
        Some(ContextPressure::High)
    );
    assert_eq!(
        m.intelligence
            .context
            .pressure(now + Duration::from_secs(121)),
        None
    );
    for suffix in [
        event("compacted", Value::Null),
        event("turn_context", serde_json::json!({"model":"changed-model"})),
    ] {
        assert_eq!(
            parse_tail(format!("{raw}\n{suffix}").as_bytes(), now)
                .2
                .intelligence
                .context,
            ContextUsage::Unknown
        );
    }
    for info in [
        Value::Null,
        serde_json::json!({"last_token_usage":{"total_tokens":-1},"model_context_window":100}),
        serde_json::json!({"last_token_usage":{"total_tokens":101},"model_context_window":100}),
        serde_json::json!({"last_token_usage":{"total_tokens":0},"model_context_window":0}),
    ] {
        let suffix = event(
            "event_msg",
            serde_json::json!({"type":"token_count","info":info}),
        );
        assert_eq!(
            parse_tail(format!("{raw}\n{suffix}").as_bytes(), now)
                .2
                .intelligence
                .context,
            ContextUsage::Unavailable
        );
    }
    let after = event(
        "event_msg",
        serde_json::json!({"type":"token_count", "info":{"last_token_usage":{"total_tokens":12000}, "model_context_window":100000}}),
    );
    assert!(matches!(
        parse_tail(
            format!("{raw}\n{}\n{after}", event("compacted", Value::Null)).as_bytes(),
            now
        )
        .2
        .intelligence
        .context,
        ContextUsage::Observed { fraction: 0.12, .. }
    ));
    let future = count.replace("08:59:59", "09:01:00");
    assert_eq!(
        parse_tail(future.as_bytes(), now).2.intelligence.context,
        ContextUsage::Unknown
    );
    assert!(parse_tail(b"partial", now).2.intelligence.model.is_none());
}

#[test]
fn long_turn_keeps_reported_model_and_workspace() {
    use std::io::Write;
    let path = std::env::temp_dir().join(format!("verge-codex-long-{}.jsonl", std::process::id()));
    let now = timestamp("2026-09-08T09:00:00Z").unwrap();
    let mut file = fs::File::create(&path).unwrap();
    writeln!(file,r#"{{"timestamp":"2026-09-08T08:59:58Z","type":"turn_context","payload":{{"model":"fixture-model","cwd":"D:/Fixture"}}}}"#).unwrap();
    // Synthetic irrelevant payload pushes metadata outside the ordinary hot tail.
    writeln!(file, "{}", "x".repeat(600 * 1024)).unwrap();
    writeln!(file,r#"{{"timestamp":"2026-09-08T08:59:59Z","type":"event_msg","payload":{{"type":"token_count","info":{{"last_token_usage":{{"total_tokens":82000}},"model_context_window":100000}}}}}}"#).unwrap();
    drop(file);
    let result = read_rollout(&mut fs::File::open(&path).unwrap(), now).unwrap();
    fs::remove_file(path).unwrap();
    assert_eq!(
        result
            .2
            .intelligence
            .model
            .as_ref()
            .map(|m| m.name.as_str()),
        Some("fixture-model")
    );
    assert_eq!(result.2.name.as_deref(), Some("Fixture"));
    assert_eq!(
        result.2.intelligence.context.pressure(now),
        Some(ContextPressure::High)
    );
}
