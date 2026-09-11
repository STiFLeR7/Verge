//! Contract test: pins how `ClaudeCodeUsageSource` interprets a recorded
//! local-state fixture. Uses a fake `CredentialStore` and a temp-file copy
//! of a real (structurally) `stats-cache.json` shape — never a real
//! credential, per docs/PRODUCT_ARCHITECTURE.md §19.

use verge_core::domain::{Account, Availability, ToolId, UsageReading};
use verge_core::ports::{CredentialOutcome, CredentialStore, SecretHandle, UsageSource};
use verge_tool_claude::ClaudeCodeUsageSource;

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
        tool: ToolId::ClaudeCode,
        label: "Claude Code".into(),
        provenance: "test".into(),
    }
}

fn write_fixture(json: &str) -> tempfile_path::TempJsonFile {
    tempfile_path::TempJsonFile::new(json)
}

#[test]
fn no_credential_is_unauthenticated_not_an_error() {
    let store = FakeCredentialStore {
        outcome: || CredentialOutcome::NotFound,
    };
    let fixture = write_fixture(r#"{"dailyActivity": []}"#);
    let source = ClaudeCodeUsageSource::new(store, fixture.path());

    assert_eq!(source.fetch(&account()), Err(Availability::Unauthenticated));
}

#[test]
fn access_denied_credential_is_access_denied_not_unauthenticated() {
    let store = FakeCredentialStore {
        outcome: || CredentialOutcome::AccessDenied,
    };
    let fixture = write_fixture(r#"{"dailyActivity": []}"#);
    let source = ClaudeCodeUsageSource::new(store, fixture.path());

    assert_eq!(source.fetch(&account()), Err(Availability::AccessDenied));
}

#[test]
fn credential_present_but_no_activity_recorded_is_not_metered() {
    let store = FakeCredentialStore {
        outcome: || CredentialOutcome::Found(SecretHandle::new("{}".to_string())),
    };
    let fixture = write_fixture(r#"{"dailyActivity": []}"#);
    let source = ClaudeCodeUsageSource::new(store, fixture.path());

    assert_eq!(source.fetch(&account()), Err(Availability::NotMetered));
}

#[test]
fn real_shaped_fixture_yields_a_bare_count_never_a_fraction() {
    let store = FakeCredentialStore {
        outcome: || CredentialOutcome::Found(SecretHandle::new("{}".to_string())),
    };
    // Structurally identical to a real `stats-cache.json` entry (see
    // docs/design/claude-code-windows-local-state.md) — no real user data.
    let fixture = write_fixture(
        r#"{"dailyActivity": [
            {"date": "2026-03-04", "messageCount": 540, "sessionCount": 7, "toolCallCount": 56},
            {"date": "2026-03-05", "messageCount": 2432, "sessionCount": 14, "toolCallCount": 350}
        ]}"#,
    );
    let source = ClaudeCodeUsageSource::new(store, fixture.path());

    let snapshot = source.fetch(&account()).expect("expected Available");
    assert_eq!(snapshot.availability, Availability::Available);
    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(snapshot.windows[0].reading, UsageReading::Count(2432));
    assert!(snapshot.windows[0].name.contains("2026-03-05"));
}

/// Tiny local temp-file helper — deliberately not a dependency, this is the
/// entire need.
mod tempfile_path {
    use std::io::Write;
    use std::path::PathBuf;

    pub struct TempJsonFile {
        path: PathBuf,
    }

    impl TempJsonFile {
        pub fn new(json: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "verge-claude-contract-{}-{}.json",
                std::process::id(),
                unique_id()
            ));
            let mut file = std::fs::File::create(&path).expect("create temp fixture");
            file.write_all(json.as_bytes()).expect("write temp fixture");
            Self { path }
        }

        pub fn path(&self) -> PathBuf {
            self.path.clone()
        }
    }

    impl Drop for TempJsonFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn unique_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }
}
