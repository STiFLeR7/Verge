use std::path::PathBuf;

use verge_core::ports::{CredentialOutcome, CredentialStore, SecretHandle};

/// **Empirical finding, not an assumption** (see
/// docs/design/codex-linux-local-state.md): a standard Codex CLI install
/// stores its credential as a plaintext file at `$CODEX_HOME/auth.json`,
/// falling back to `$HOME/.codex/auth.json` when `CODEX_HOME` is unset —
/// confirmed directly from `codex-rs`'s own `find_codex_home` source, not
/// inferred from the Windows Codex installation observed in a prior
/// session. Exactly like `WindowsCredentialStore`, this is a per-known-key
/// file lookup, not a wrapper around a real OS secret store (Secret
/// Service/GNOME Keyring) — Codex's own default `AuthCredentialsStoreMode`
/// is `File`, not `Keyring`, so there is nothing else to wrap for the
/// default install this adapter targets.
pub struct LinuxCredentialStore {
    known_paths: Vec<(&'static str, PathBuf)>,
}

impl LinuxCredentialStore {
    pub fn discover() -> Self {
        let mut known_paths = Vec::new();
        if let Some(codex_home) = codex_home() {
            known_paths.push(("codex", codex_home.join("auth.json")));
        }
        Self { known_paths }
    }
}

fn codex_home() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CODEX_HOME") {
        return Some(PathBuf::from(dir));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex"))
}

impl CredentialStore for LinuxCredentialStore {
    fn find(&self, key: &str) -> CredentialOutcome {
        let Some((_, path)) = self.known_paths.iter().find(|(k, _)| *k == key) else {
            return CredentialOutcome::NotFound;
        };

        match std::fs::read_to_string(path) {
            Ok(raw) => CredentialOutcome::Found(SecretHandle::new(raw)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => CredentialOutcome::NotFound,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                CredentialOutcome::AccessDenied
            }
            Err(e) => CredentialOutcome::StoreUnavailable {
                reason: e.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_key_is_not_found() {
        let store = LinuxCredentialStore {
            known_paths: vec![],
        };
        assert!(matches!(
            store.find("some-other-tool"),
            CredentialOutcome::NotFound
        ));
    }

    #[test]
    fn missing_file_for_known_key_is_not_found_not_error() {
        let store = LinuxCredentialStore {
            known_paths: vec![(
                "codex",
                PathBuf::from("/definitely/does/not/exist/auth.json"),
            )],
        };
        assert!(matches!(store.find("codex"), CredentialOutcome::NotFound));
    }
}
