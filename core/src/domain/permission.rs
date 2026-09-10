use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    pub id: u64,
    pub session_id: String,
    pub tool: String,
    pub detail: String,
    pub cwd: String,
    pub created: Instant,
    pub expires: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionDecision {
    Approve,
    Deny,
    Dismiss,
}

struct Entry {
    request: PermissionRequest,
    decision: Option<PermissionDecision>,
}
#[derive(Default)]
pub struct PermissionBook {
    next: u64,
    entries: BTreeMap<u64, Entry>,
}
impl PermissionBook {
    pub fn register(
        &mut self,
        session_id: String,
        tool: String,
        detail: String,
        cwd: String,
        now: Instant,
    ) -> Option<u64> {
        if self.entries.len() >= 8
            || session_id.is_empty()
            || detail.is_empty()
            || detail.len() > 16384
        {
            return None;
        }
        self.next = self.next.checked_add(1)?;
        let id = self.next;
        self.entries.insert(
            id,
            Entry {
                request: PermissionRequest {
                    id,
                    session_id,
                    tool,
                    detail,
                    cwd,
                    created: now,
                    expires: now + Duration::from_secs(120),
                },
                decision: None,
            },
        );
        Some(id)
    }
    pub fn pending(&self, now: Instant) -> Option<PermissionRequest> {
        self.entries
            .values()
            .find(|e| e.decision.is_none() && now < e.request.expires)
            .map(|e| e.request.clone())
    }
    pub fn decide(
        &mut self,
        id: u64,
        session: &str,
        decision: PermissionDecision,
        now: Instant,
        live: bool,
    ) -> bool {
        let Some(e) = self.entries.get_mut(&id) else {
            return false;
        };
        if !live
            || e.request.session_id != session
            || e.decision.is_some()
            || now >= e.request.expires
        {
            return false;
        }
        e.decision = Some(decision);
        true
    }
    pub fn result(&self, id: u64, now: Instant) -> Option<PermissionDecision> {
        self.entries.get(&id).and_then(|e| {
            if now >= e.request.expires {
                Some(PermissionDecision::Deny)
            } else {
                e.decision
            }
        })
    }
    pub fn remove(&mut self, id: u64) {
        self.entries.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permission_lifecycle_is_bound_to_one_live_request_and_session() {
        let now = Instant::now();
        let mut b = PermissionBook::default();
        let add = |b: &mut PermissionBook, s: &str| {
            b.register(
                s.into(),
                "Bash".into(),
                "echo test".into(),
                "test-project".into(),
                now,
            )
            .unwrap()
        };
        let first = add(&mut b, "a");
        let other = add(&mut b, "b");
        assert_eq!(b.pending(now).unwrap().id, first); // appears
        assert!(!b.decide(999, "a", PermissionDecision::Approve, now, true)); // unknown
        assert!(!b.decide(first, "b", PermissionDecision::Approve, now, true)); // other session
        assert!(!b.decide(first, "a", PermissionDecision::Approve, now, false)); // terminated
        assert!(b.decide(first, "a", PermissionDecision::Approve, now, true));
        assert_eq!(b.result(first, now), Some(PermissionDecision::Approve));
        assert!(!b.decide(first, "a", PermissionDecision::Deny, now, true)); // duplicate
        assert_eq!(b.pending(now).unwrap().id, other);
        b.remove(first);
        let newer = add(&mut b, "a");
        assert_ne!(first, newer);
        assert!(!b.decide(first, "a", PermissionDecision::Approve, now, true)); // stale UI
        assert!(b.decide(other, "b", PermissionDecision::Deny, now, true));
        assert_eq!(b.result(other, now), Some(PermissionDecision::Deny));
        assert!(b.decide(newer, "a", PermissionDecision::Dismiss, now, true));
        assert_ne!(b.result(newer, now), Some(PermissionDecision::Approve));
        let expired = add(&mut b, "a");
        let later = now + Duration::from_secs(121);
        assert!(!b.decide(expired, "a", PermissionDecision::Approve, later, true));
        assert_eq!(b.result(expired, later), Some(PermissionDecision::Deny));
        let canceled = add(&mut b, "a");
        b.remove(canceled);
        assert!(!b.decide(canceled, "a", PermissionDecision::Approve, now, true));
    }
}

impl std::fmt::Debug for PermissionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PermissionRequest(<private>)")
    }
}
