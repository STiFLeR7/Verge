//! Portable read-only local data composition; native shells never interpret tool logs.
use verge_core::domain::{project_ambient_state, Account, AmbientState, ToolId};

pub fn local_states() -> Vec<AmbientState> {
    let mut states = Vec::new();
    if let Some(path) = verge_tool_codex::local::default_sessions_path() {
        states.push(verge_tool_codex::local::read_state(&path));
    }
    let account = Account {
        tool: ToolId::ClaudeCode,
        label: "Claude".into(),
        provenance: "Local status-line metadata".into(),
    };
    if let Some(snapshot) = verge_tool_claude::read_limits(&account) {
        states.push(project_ambient_state(&snapshot));
    }
    states
}

/// Versioned presentation-only bridge for the minimal AppKit shim. No credentials/commands.
pub fn snapshot() -> serde_json::Value {
    snapshot_for(&local_states())
}

// Ephemeral view identity only, never an authorization token or persisted identifier.
fn view_id(id: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

fn snapshot_for(states: &[AmbientState]) -> serde_json::Value {
    use verge_core::ports::Metric;
    let content = verge_ui_ambient_shared::render_requested(states, true);
    serde_json::json!({"version": 1, "tools": content.glyphs.iter().map(|g| serde_json::json!({
        "name":g.label, "activity":g.activity_label, "state":format!("{:?}",g.state),
        "usage":match g.metric {Metric::Fraction(f)=>Some(f), _=>None},
        "summary":g.session_summary, "sessions":g.sessions.iter().map(|s|serde_json::json!({"id":view_id(&s.id),"title":s.title,"lines":s.lines})).collect::<Vec<_>>(),
        "limits":g.usage_windows.iter().map(|w|serde_json::json!({"name":w.label,"fraction":w.fraction,"reset":w.reset})).collect::<Vec<_>>()
    })).collect::<Vec<_>>()})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_preserves_unknown_usage_without_exposing_source_metadata() {
        use verge_core::domain::{ActivityState, Availability};
        let state = AmbientState {
            account: Account {
                tool: ToolId::Codex,
                label: "private account".into(),
                provenance: "NEVER EXPOSE".into(),
            },
            availability: Availability::Unsupported {
                reason: "private path".into(),
            },
            most_constrained_window: None,
            usage_windows: vec![],
            reminder: None,
            sessions: None,
            activity: ActivityState::Unknown,
        };
        let value = snapshot_for(&[state]);
        assert_eq!(value["version"], 1);
        assert_eq!(value["tools"][0]["name"], "ChatGPT");
        assert!(value["tools"][0]["usage"].is_null());
        assert!(value["tools"][0]["sessions"].as_array().unwrap().is_empty());
        assert_eq!(view_id("/private/session"), view_id("/private/session"));
        assert_ne!(view_id("/private/session"), view_id("/private/other"));
        assert!(!view_id("/private/session").contains("private"));
        let raw = value.to_string();
        assert!(!raw.contains("private") && !raw.contains("NEVER EXPOSE"));
        assert_eq!(snapshot_for(&[])["tools"], serde_json::json!([]));
    }
}
