/// An opaque secret, held in memory only. Deliberately has no `Display`
/// and a redacted `Debug` — nothing that touches this type may format its
/// contents into a log line. Caching a `SecretHandle` across calls, and any
/// TTL policy for that cache, is the calling application layer's job, not
/// this contract's (mirrors the reference product's `CredentialCache`).
pub struct SecretHandle(String);

impl SecretHandle {
    pub fn new(raw: String) -> Self {
        SecretHandle(raw)
    }

    /// The only way to look inside. Callers that need to parse this (a
    /// tool adapter interpreting its own vendor's token shape) must do so
    /// deliberately and must never pass the result to a logger.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretHandle(<redacted>)")
    }
}

#[derive(Debug)]
pub enum CredentialOutcome {
    Found(SecretHandle),
    NotFound,
    AccessDenied,
    /// No keyring/store daemon running, or (on this vertical slice's
    /// Windows implementation) the backing file could not be read for a
    /// reason other than "does not exist" or "permission denied".
    StoreUnavailable {
        reason: String,
    },
}

/// Read (never write) a named secret from the OS's secure store, or — on a
/// platform build where no OS secure store is used yet — from wherever the
/// tool actually keeps it. One implementation per OS (Axis B).
pub trait CredentialStore {
    fn find(&self, key: &str) -> CredentialOutcome;
}
