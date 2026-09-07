use crate::domain::{Account, Availability, UsageSnapshot};

/// Fetch one account's current `UsageSnapshot`. One implementation per
/// `Tool` (Axis A) — this port is declared here in `core/ports`, never
/// invented ad hoc inside a `tools/` implementation.
///
/// On failure, returns a typed `Availability` cause, never a raw error the
/// caller must interpret.
pub trait UsageSource {
    fn fetch(&self, account: &Account) -> Result<UsageSnapshot, Availability>;
}
