//! Codex adapters: credential-only compatibility and local rollout metadata.
//! `local` reads reported rate limits and recent task events without exposing conversation content.

pub mod local;

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
                        // Credentials alone do not contain usage; the local module reads rollout events.
                        Err(Availability::Unsupported {
                            reason: "Credential-only source does not report usage; use the local rollout adapter"
                                .to_string(),
                        })
                    }
                }
            }
        }
    }
}
