//! Contract test: pins how `CodexUsageSource` interprets its credential
//! store's outcomes. Uses a fake `CredentialStore` — never a real
//! credential — per docs/PRODUCT_ARCHITECTURE.md §19.
//!
//! Fixture origin: `docs/design/codex-linux-local-state.md`'s `AuthDotJson`
//! shape, established by direct inspection of `codex-rs` v0.153.4's own
//! source (`login/src/auth/storage.rs`). No real account identifiers.

use verge_core::domain::{Account, Availability, ToolId};
use verge_core::ports::{CredentialOutcome, CredentialStore, SecretHandle, UsageSource};
use verge_tool_codex::CodexUsageSource;

struct FakeCredentialStore {
    outcome: fn() -> CredentialOutcome,
}

impl CredentialStore for FakeCredentialStore {
    fn find(&self, _key: &str) -> CredentialOutcome {
        (self.outcome)()
    }
}

fn account() -> Account {
    Account {
        tool: ToolId::Codex,
        label: "Codex".into(),
        provenance: "test".into(),
    }
}

/// Structurally modeled on the real `AuthDotJson` shape (see the design
/// doc) — no real tokens, no real account/email identifiers.
const REALISTIC_AUTH_JSON: &str = r#"{
    "auth_mode": "chatgpt",
    "OPENAI_API_KEY": null,
    "tokens": {
        "id_token": "fixture.jwt.value",
        "access_token": "fixture.jwt.value",
        "refresh_token": "fixture-refresh-token",
        "account_id": "fixture-account-id"
    },
    "last_refresh": "2026-09-07T00:00:00Z"
}"#;

#[test]
fn missing_credential_is_unauthenticated_not_an_error() {
    let store = FakeCredentialStore {
        outcome: || CredentialOutcome::NotFound,
    };
    let source = CodexUsageSource::new(store);

    assert_eq!(source.fetch(&account()), Err(Availability::Unauthenticated));
}

#[test]
fn access_denied_credential_is_access_denied_not_unauthenticated() {
    let store = FakeCredentialStore {
        outcome: || CredentialOutcome::AccessDenied,
    };
    let source = CodexUsageSource::new(store);

    assert_eq!(source.fetch(&account()), Err(Availability::AccessDenied));
}

#[test]
fn malformed_auth_json_is_a_typed_error_not_a_fabricated_available() {
    let store = FakeCredentialStore {
        outcome: || {
            CredentialOutcome::Found(SecretHandle::new("{ this is not valid json".to_string()))
        },
    };
    let source = CodexUsageSource::new(store);

    let result = source.fetch(&account());
    assert!(
        matches!(result, Err(Availability::Error { .. })),
        "expected Error, got {result:?}"
    );
}

#[test]
fn realistic_auth_json_reports_unsupported_never_a_fabricated_usage_number() {
    let store = FakeCredentialStore {
        outcome: || CredentialOutcome::Found(SecretHandle::new(REALISTIC_AUTH_JSON.to_string())),
    };
    let source = CodexUsageSource::new(store);

    // The single most important assertion in this file: a real, valid
    // credential must never be translated into an invented usage number —
    // see docs/design/codex-linux-local-state.md's "Usage signal" finding.
    assert_eq!(
        source.fetch(&account()),
        Err(Availability::Unsupported {
            reason: "Codex has no local usage/quota cache; account usage requires \
                     a live authenticated API call, which this adapter does not \
                     perform"
                .to_string()
        })
    );
}

#[test]
fn schema_drifted_but_valid_json_is_still_tolerated() {
    // A future Codex version could rename/add fields (see the Stability
    // section of the design doc); this adapter never reads a specific
    // field, so any valid JSON object must still resolve the same way.
    let store = FakeCredentialStore {
        outcome: || {
            CredentialOutcome::Found(SecretHandle::new(
                r#"{"some_future_field": {"nested": true}, "auth_mode": "workload_identity"}"#
                    .to_string(),
            ))
        },
    };
    let source = CodexUsageSource::new(store);

    assert!(matches!(
        source.fetch(&account()),
        Err(Availability::Unsupported { .. })
    ));
}
