use super::{Account, Availability, Fidelity, Recency};
use std::time::SystemTime;

/// A window's reading: a fraction of a published limit, or a bare count
/// when no limit is published. Mirrors the reference product's honest
/// "no limit" case — never coerced into a fake percentage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsageReading {
    Fraction(f64),
    Count(u64),
}

/// For one rate-limit (or rate-limit-like) window, how much is used and
/// when does it reset.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageWindow {
    pub name: String,
    pub reading: UsageReading,
    pub resets_at: Option<SystemTime>,
    pub fidelity: Fidelity,
    pub recency: Recency,
}

/// What we currently know about one account's usage, as of when.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageSnapshot {
    pub account: Account,
    pub availability: Availability,
    pub windows: Vec<UsageWindow>,
}

/// Three-state description of what one unit of live/recent agent work is
/// doing right now. Activity is a separate concern from usage — it updates
/// on its own cadence and must never be inferred from a usage poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityState {
    Working,
    WaitingOnUser,
    RecentlyIdle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivitySession {
    pub state: ActivityState,
    /// What it's waiting for, if `state` is `WaitingOnUser`.
    pub waiting_for: Option<String>,
    pub since: SystemTime,
}
