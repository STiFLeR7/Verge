//! Codex tool adapter.
//!
//! Owns interpretation of Codex's local state: which file is relevant, what
//! it means for a real credential to be present, and — the one genuinely
//! different finding from Claude Code — that Codex has **no local
//! usage/quota cache to read at all** (see
//! docs/design/codex-linux-local-state.md). This adapter contains zero OS
//! code (no raw file-descriptor tricks, no `/proc`, no X11/Win32); the
//! `CredentialStore` it depends on is injected by whichever platform crate
//! composes it.

use verge_core::domain::{Account, Availability, UsageSnapshot};
use verge_core::ports::{CredentialOutcome, CredentialStore, UsageSource};

/// The key this adapter asks its injected `CredentialStore` for. What that
/// key resolves to (a file path today) is entirely the store's business —
/// this adapter never constructs a path itself.
pub const CREDENTIAL_KEY: &str = "codex";

pub struct CodexUsageSource<C: CredentialStore> {
    credential_store: C,
}

impl<C: CredentialStore> CodexUsageSource<C> {
    pub fn new(credential_store: C) -> Self {
        Self { credential_store }
    }
}

impl<C: CredentialStore> UsageSource for CodexUsageSource<C> {
    fn fetch(&self, _account: &Account) -> Result<UsageSnapshot, Availability> {
        match self.credential_store.find(CREDENTIAL_KEY) {
            CredentialOutcome::NotFound => Err(Availability::Unauthenticated),
            CredentialOutcome::AccessDenied => Err(Availability::AccessDenied),
            CredentialOutcome::StoreUnavailable { reason } => {
                Err(Availability::Unsupported { reason })
            }
            CredentialOutcome::Found(handle) => {
                // We only ever check that this parses as a JSON object — a
                // sanity check against a corrupted/partially-written file —
                // and never read a specific field out of it. Codex's own
                // `AuthDotJson` shape (docs/design/codex-linux-local-state.md)
                // is intentionally not modeled here: this adapter has no use
                // for any field in it, and modeling fields we never read
                // would be exactly the kind of unused abstraction the
                // architecture warns against.
                match serde_json::from_str::<serde_json::Value>(handle.expose()) {
                    Err(e) => Err(Availability::Error {
                        diagnostic: format!("auth.json did not parse as JSON: {e}"),
                    }),
                    Ok(_) => {
                        // CONFIRMED by direct source inspection (see the
                        // design doc): Codex's rate-limit/usage data comes
                        // only from live HTTP response headers on an
                        // authenticated API call. There is no local file or
                        // database this adapter can read instead, so
                        // reporting anything other than `Unsupported` here
                        // would be inventing a number — exactly what
                        // docs/PRODUCT_ARCHITECTURE.md §6 forbids.
                        Err(Availability::Unsupported {
                            reason: "Codex has no local usage/quota cache; account usage requires \
                                     a live authenticated API call, which this adapter does not \
                                     perform"
                                .to_string(),
                        })
                    }
                }
            }
        }
    }
}
