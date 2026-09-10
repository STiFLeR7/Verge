use crate::domain::{ActivitySession, Availability};
use std::time::SystemTime;

pub trait ActivitySource {
    fn sessions(&self) -> Result<Vec<ActivitySession>, Availability>;
}

/// The platform verifies process identity; tool adapters interpret session records.
pub trait ProcessProbe {
    fn started_at(&self, pid: u32) -> std::io::Result<Option<SystemTime>>;
}
