use std::{
    fs,
    time::{Duration, SystemTime},
};
use verge_core::ports::ProcessProbe;

/// Rechecks the exact Claude session file against the live ancestor process.
pub fn permission_session_matches(session: &str, pid: u32, probe: &impl ProcessProbe) -> bool {
    let check = || -> Option<()> {
        if session.is_empty()
            || session.len() > 100
            || !session
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return None;
        }
        let path = crate::default_sessions_path()?.join(format!("{pid}.json"));
        if fs::metadata(&path).ok()?.len() > 65536 {
            return None;
        }
        let v: serde_json::Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        if v["sessionId"].as_str() != Some(session) || v["pid"].as_u64() != Some(pid as u64) {
            return None;
        }
        let ticks = v["procStart"]
            .as_str()?
            .parse::<u64>()
            .ok()?
            .checked_sub(116_444_736_000_000_000)?
            .checked_mul(100)?;
        let expected = SystemTime::UNIX_EPOCH.checked_add(Duration::from_nanos(ticks))?;
        (probe.started_at(pid).ok()?? == expected).then_some(())
    };
    check().is_some()
}

pub struct ClaudePermissionInput {
    pub session: String,
    pub tool: String,
    pub detail: String,
    pub cwd: String,
}
pub fn parse_permission(raw: &[u8]) -> Option<ClaudePermissionInput> {
    if raw.len() > 32768 {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(raw).ok()?;
    if v["hook_event_name"] != "PermissionRequest" {
        return None;
    }
    let session = v["session_id"].as_str()?.to_owned();
    let tool = v["tool_name"].as_str()?.to_owned();
    let cwd = v["cwd"].as_str()?.to_owned();
    let input = v.get("tool_input")?;
    if !input.is_object() || tool.len() > 100 || cwd.len() > 1024 {
        return None;
    }
    // Full arguments, not a tool-supplied description that could conceal the operation.
    let detail = serde_json::to_string_pretty(input).ok()?;
    if detail.len() > 16384 {
        return None;
    }
    Some(ClaudePermissionInput {
        session,
        tool,
        detail,
        cwd,
    })
}
pub fn permission_output(allow: bool) -> String {
    let decision = if allow {
        serde_json::json!({"behavior":"allow"})
    } else {
        serde_json::json!({"behavior":"deny","message":"Not approved in Verge. You can ask again in Claude."})
    };
    serde_json::json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":decision}}).to_string()
}

#[cfg(test)]
#[test]
fn only_permission_requests_produce_supported_explicit_decisions() {
    let raw=br#"{"hook_event_name":"PermissionRequest","session_id":"test","tool_name":"Bash","cwd":"test-project","tool_input":{"command":"echo test","description":"friendly description"}}"#;
    let r = parse_permission(raw).unwrap();
    assert!(r.detail.contains("echo test"));
    assert!(parse_permission(b"{}").is_none());
    for allow in [false, true] {
        let v: serde_json::Value = serde_json::from_str(&permission_output(allow)).unwrap();
        assert_eq!(
            v["hookSpecificOutput"]["hookEventName"],
            "PermissionRequest"
        );
        assert_eq!(
            v["hookSpecificOutput"]["decision"]["behavior"],
            if allow { "allow" } else { "deny" }
        );
        assert!(v["hookSpecificOutput"]["decision"]
            .get("updatedPermissions")
            .is_none());
    }
}
