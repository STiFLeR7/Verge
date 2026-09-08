//! Claude Code tool adapter.
//!
//! Owns interpretation of Claude Code's local state: which files/paths are
//! relevant, their schema, and how raw values map into domain types. Does
//! *not* own how those files are opened or how the OS credential is
//! located — that is injected as a `CredentialStore` (see
//! docs/design/claude-code-windows-local-state.md for how the paths below
//! were established empirically, not assumed from the macOS reference).

mod stats_cache;

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use verge_core::domain::{
    Account, Availability, Fidelity, Recency, UsageReading, UsageSnapshot, UsageWindow,
};
use verge_core::ports::{CredentialOutcome, CredentialStore, UsageSource};

/// The key this adapter asks its injected `CredentialStore` for. What that
/// key resolves to (a file path today; the OS's real secure store on a
/// future platform build) is entirely the store's business.
pub const CREDENTIAL_KEY: &str = "claude-code";

/// A reading is still `Recent` (not `Aged`) if the source file was written
/// within this window.
const RECENT_WITHIN: Duration = Duration::from_secs(15 * 60);

pub struct ClaudeCodeUsageSource<C: CredentialStore> {
    credential_store: C,
    /// Path to `stats-cache.json`. Injectable for tests; production
    /// callers use `default_stats_cache_path()`.
    stats_cache_path: PathBuf,
}

impl<C: CredentialStore> ClaudeCodeUsageSource<C> {
    pub fn new(credential_store: C, stats_cache_path: PathBuf) -> Self {
        Self {
            credential_store,
            stats_cache_path,
        }
    }
}

/// `~/.claude/stats-cache.json` on every OS this tool has been observed on
/// so far, including Windows (verified directly against a real installed
/// Claude Code on this machine — see docs/design/claude-code-windows-local-state.md).
pub fn default_stats_cache_path() -> Option<PathBuf> {
    dirs_home().map(|home| home.join(".claude").join("stats-cache.json"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

impl<C: CredentialStore> UsageSource for ClaudeCodeUsageSource<C> {
    fn fetch(&self, account: &Account) -> Result<UsageSnapshot, Availability> {
        match self.credential_store.find(CREDENTIAL_KEY) {
            CredentialOutcome::NotFound => return Err(Availability::Unauthenticated),
            CredentialOutcome::AccessDenied => return Err(Availability::AccessDenied),
            CredentialOutcome::StoreUnavailable { reason } => {
                return Err(Availability::Unsupported { reason })
            }
            CredentialOutcome::Found(_handle) => {
                // The vertical slice only needs to know a credential
                // exists; it never reads the token value. A future
                // official-endpoint UsageSource is what would actually
                // `expose()` it.
            }
        }

        let raw =
            std::fs::read_to_string(&self.stats_cache_path).map_err(|e| Availability::Error {
                diagnostic: format!("reading stats-cache.json: {e}"),
            })?;
        let modified = std::fs::metadata(&self.stats_cache_path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        let stats: stats_cache::StatsCache =
            serde_json::from_str(&raw).map_err(|e| Availability::Error {
                diagnostic: format!("parsing stats-cache.json: {e}"),
            })?;

        let windows = match stats.most_recent_day_message_count() {
            Some((date, count)) => vec![UsageWindow {
                // No published limit is available from local state alone,
                // so this is honestly a bare count, not a fabricated
                // fraction — see docs/PRODUCT_ARCHITECTURE.md §6. Labeled
                // with the actual date the count is for, since this
                // locally-cached file can lag well behind "today" (observed
                // directly on this machine) and claiming "today" for a
                // stale entry would misrepresent recency as fidelity.
                name: format!("messages on {date} (local count, no published limit)"),
                reading: UsageReading::Count(count),
                resets_at: None,
                fidelity: Fidelity::Derived,
                recency: Recency::classify(modified, SystemTime::now(), RECENT_WITHIN),
            }],
            // A tool with no activity recorded yet is `NotMetered`, not a
            // fabricated zero-usage window.
            None => return Err(Availability::NotMetered),
        };

        Ok(UsageSnapshot {
            account: account.clone(),
            availability: Availability::Available,
            windows,
        })
    }
}
