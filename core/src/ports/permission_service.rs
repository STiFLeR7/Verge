use crate::domain::{PermissionDecision, PermissionRequest};

pub trait PermissionService: Send + Sync {
    fn pending(&self) -> Option<PermissionRequest>;
    fn decide(&self, id: u64, session: &str, decision: PermissionDecision) -> bool;
}
