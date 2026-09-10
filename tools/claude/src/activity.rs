use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use verge_core::{
    domain::{ActivitySession, ActivityState, Availability},
    ports::{ActivitySource, ProcessProbe},
};

pub struct ClaudeActivitySource<P> {
    directory: PathBuf,
    processes: P,
    contexts: std::sync::Mutex<BTreeMap<String, verge_core::domain::SessionIntelligence>>,
    signals: Option<PathBuf>,
}
impl<P: ProcessProbe> ClaudeActivitySource<P> {
    pub fn new(directory: PathBuf, processes: P) -> Self {
        Self {
            directory,
            processes,
            contexts: std::sync::Mutex::new(BTreeMap::new()),
            signals: crate::default_signal_directory(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Probe;
    impl ProcessProbe for Probe {
        fn started_at(&self, pid: u32) -> std::io::Result<Option<SystemTime>> {
            Ok((pid != 99).then_some(SystemTime::UNIX_EPOCH + Duration::from_secs(100)))
        }
    }
    #[test]
    fn verifies_process_identity_and_reads_only_session_metadata() {
        let dir = std::env::temp_dir().join(format!("verge-activity-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut source = ClaudeActivitySource::new(dir.clone(), Probe);
        source.signals = Some(dir.join("signals"));
        assert!(source.sessions().unwrap().is_empty());
        let record = |pid, status, start| {
            serde_json::json!({"pid":pid,"cwd":"D:\\Verge","status":status,"procStart":start,"statusUpdatedAt":105000}).to_string()
        };
        let start = "116444737000000000";
        fs::write(dir.join("1.json"), record(1, "busy", start)).unwrap();
        fs::write(dir.join("99.json"), record(99, "busy", start)).unwrap();
        fs::write(dir.join("2.json"), record(2, "busy", "116444738000000000")).unwrap();
        fs::write(dir.join("ignored.tmp"), "partial").unwrap();
        let sessions = source.sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "Verge");
        assert_eq!(sessions[0].state, ActivityState::Working);
        for (raw, state) in [
            ("waiting", ActivityState::WaitingOnUser),
            ("idle", ActivityState::RecentlyIdle),
            ("stopped", ActivityState::Stopped),
            ("future", ActivityState::Unknown),
        ] {
            fs::write(dir.join("1.json"), record(1, raw, start)).unwrap();
            assert_eq!(source.sessions().unwrap()[0].state, state);
        }
        fs::create_dir_all(source.signals.as_ref().unwrap()).unwrap();
        let mut meta: serde_json::Value = serde_json::from_str(&record(1, "busy", start)).unwrap();
        meta["sessionId"] = "test-context-session".into();
        fs::write(dir.join("1.json"), meta.to_string()).unwrap();
        let context_path = source
            .signals
            .as_ref()
            .unwrap()
            .join("claude-context-test-context-session.json");
        let context = serde_json::json!({"session_id":"test-context-session","at":105000,"model":{"id":"test-model","name":"Test model"},"context":{"used_percentage":82,"capacity":200000}});
        fs::write(&context_path, context.to_string()).unwrap();
        let first = source.sessions().unwrap()[0].intelligence.clone();
        assert!(first.is_some());
        fs::write(&context_path, "{").unwrap();
        assert_eq!(
            source.sessions().unwrap()[0].intelligence,
            first,
            "last good observation keeps original timestamp"
        );
        let mut cleared = context.clone();
        cleared["context"] = serde_json::Value::Null;
        fs::write(&context_path, cleared.to_string()).unwrap();
        assert_eq!(
            source.sessions().unwrap()[0]
                .intelligence
                .as_ref()
                .unwrap()
                .context,
            verge_core::domain::ContextUsage::Unavailable
        );
        fs::write(dir.join("duplicate.json"), meta.to_string()).unwrap();
        assert_eq!(
            source.sessions().unwrap().len(),
            1,
            "duplicate records do not multiply sessions"
        );
        fs::remove_file(dir.join("duplicate.json")).unwrap();
        let mut collision = meta.clone();
        collision["pid"] = 2.into();
        fs::write(dir.join("2.json"), collision.to_string()).unwrap();
        let colliding = source.sessions().unwrap();
        assert_eq!(colliding.len(), 2);
        assert_ne!(colliding[0].id, colliding[1].id);
        assert!(
            colliding.iter().all(|s| s.intelligence.is_none()),
            "ambiguous context never attaches across live processes"
        );
        fs::remove_file(dir.join("2.json")).unwrap();
        fs::write(dir.join("1.json"), "{").unwrap();
        assert!(
            source.sessions().is_err(),
            "partial metadata is unknown, not zero sessions"
        );
        fs::remove_dir_all(dir).unwrap();
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    pid: u32,
    cwd: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    tempo: Option<String>,
    #[serde(default)]
    proc_start: Option<String>,
    #[serde(default)]
    started_at: Option<u64>,
    #[serde(default)]
    status_updated_at: Option<u64>,
    #[serde(default)]
    updated_at: Option<u64>,
    #[serde(default)]
    waiting_for: Option<String>,
    #[serde(default)]
    needs: Option<String>,
}

impl<P: ProcessProbe> ActivitySource for ClaudeActivitySource<P> {
    fn sessions(&self) -> Result<Vec<ActivitySession>, Availability> {
        let entries = match fs::read_dir(&self.directory) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(_) => return Err(Availability::AccessDenied),
        };
        let mut sessions = BTreeMap::<String, ActivitySession>::new();
        let mut claimed_ids = BTreeMap::<String, String>::new();
        for entry in entries {
            let entry = entry.map_err(|_| Availability::AccessDenied)?;
            if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            // Bounded reads: records are small metadata, never transcripts or credentials.
            let mut raw = Vec::new();
            let file = fs::File::open(entry.path()).map_err(|_| Availability::AccessDenied)?;
            if file.take(65_537).read_to_end(&mut raw).is_err() || raw.len() > 65_536 {
                return Err(Availability::Unreachable);
            }
            let record =
                serde_json::from_slice::<Record>(&raw).map_err(|_| Availability::Unreachable)?;
            if record.pid == 0 {
                continue;
            }
            let Some(started) = self
                .processes
                .started_at(record.pid)
                .map_err(|_| Availability::AccessDenied)?
            else {
                continue;
            };
            // Windows procStart is a FILETIME string, not macOS's ctime string.
            let expected = record
                .proc_start
                .as_deref()
                .and_then(|s| s.parse::<u64>().ok())
                .and_then(|n| n.checked_sub(116_444_736_000_000_000))
                .and_then(|n| n.checked_mul(100))
                .and_then(|n| SystemTime::UNIX_EPOCH.checked_add(Duration::from_nanos(n)));
            if let Some(expected) = expected {
                if started != expected {
                    continue;
                }
            } else if let Some(ms) = record.started_at {
                let Some(expected) = SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(ms))
                else {
                    continue;
                };
                let difference = started
                    .duration_since(expected)
                    .or_else(|_| expected.duration_since(started))
                    .unwrap_or_default();
                if difference > Duration::from_secs(5) {
                    continue;
                }
            } else {
                continue;
            } // Unverified PID reuse must not invent a live session.
            let state = match (record.tempo.as_deref(), record.status.as_deref()) {
                (Some("blocked"), _) | (_, Some("waiting")) => ActivityState::WaitingOnUser,
                (Some("active"), _) | (_, Some("busy")) => ActivityState::Working,
                (_, Some("idle")) => ActivityState::RecentlyIdle,
                (_, Some("completed")) => ActivityState::Completed,
                (_, Some("stopped" | "error")) => ActivityState::Stopped,
                _ => ActivityState::Unknown,
            };
            let since = record
                .status_updated_at
                .or(record.updated_at)
                .and_then(|ms| SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(ms)))
                .unwrap_or(started);
            let folder = record
                .cwd
                .trim_end_matches(['\\', '/'])
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or("Claude")
                .to_string();
            let mut session = ActivitySession {
                intelligence: None,
                id: format!(
                    "claude:{}:{}",
                    record.pid,
                    started
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                ),
                name: record.name.filter(|s| !s.is_empty()).unwrap_or(folder),
                state,
                since,
                waiting_for: record.waiting_for.or(record.needs),
            };
            if let Some(id) = record.session_id.filter(|id| {
                !id.is_empty()
                    && id.len() <= 100
                    && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            }) {
                session.id = format!(
                    "claude:{id}:{}:{}",
                    record.pid,
                    started
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos()
                );
                if let Some(dir) = &self.signals {
                    let mut contexts = self.contexts.lock().unwrap();
                    let context_path = dir.join(format!("claude-context-{id}.json"));
                    let raw = fs::File::open(context_path).ok().and_then(|file| {
                        let mut raw = Vec::new();
                        file.take(4097).read_to_end(&mut raw).ok()?;
                        (raw.len() <= 4096).then_some(raw)
                    });
                    if let Some(data) = raw
                        .as_deref()
                        .and_then(|r| crate::context::parse(r, &id, started, SystemTime::now()))
                    {
                        contexts.insert(session.id.clone(), data);
                    }
                    session.intelligence = contexts.get(&session.id).cloned();
                    let path = dir.join(format!("claude-session-{id}.json"));
                    if fs::metadata(&path).is_ok_and(|m| m.len() <= 4096) {
                        if let Ok(raw) = fs::read(path) {
                            if let Ok(event) = serde_json::from_slice::<serde_json::Value>(&raw) {
                                if let Some(at) = event["at"].as_u64().and_then(|ms| {
                                    SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(ms))
                                }) {
                                    if at >= started
                                        && (at >= session.since
                                            || (session.state == ActivityState::RecentlyIdle
                                                && event["state"].as_str() == Some("completed")))
                                        && at <= SystemTime::now() + Duration::from_secs(30)
                                    {
                                        session.state = match event["state"].as_str() {
                                            Some("busy") => ActivityState::Working,
                                            Some("waiting") => ActivityState::WaitingOnUser,
                                            Some("completed") => ActivityState::Completed,
                                            Some("stopped" | "error") => ActivityState::Stopped,
                                            _ => session.state,
                                        };
                                        session.since = at;
                                        session.waiting_for =
                                            event["waiting_for"].as_str().map(str::to_owned);
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(other) = claimed_ids.insert(id, session.id.clone()) {
                    if other != session.id {
                        // A reused tool ID cannot safely associate a shared context file.
                        session.intelligence = None;
                        if let Some(previous) = sessions.get_mut(&other) {
                            previous.intelligence = None;
                        }
                    }
                }
            }
            if sessions
                .get(&session.id)
                .is_none_or(|old: &ActivitySession| session.since > old.since)
            {
                sessions.insert(session.id.clone(), session);
            }
        }
        self.contexts
            .lock()
            .unwrap()
            .retain(|id, _| sessions.contains_key(id));
        Ok(sessions.into_values().collect())
    }
}
