use std::path::PathBuf;

use verge_core::ports::{CredentialOutcome, CredentialStore, SecretHandle};

/// **Empirical finding, not an assumption:** on Windows, Claude Code does
/// not put its OAuth token in Windows Credential Manager — it writes a
/// plaintext JSON file to `%USERPROFILE%\.claude\.credentials.json`
/// (confirmed directly against a real, in-use installation; see
/// docs/design/claude-code-windows-local-state.md). This implementation
/// therefore does not wrap `Win32 CredRead`/Credential Manager at all —
/// there is nothing there to wrap for this tool. It is a per-known-key
/// lookup to a local file, kept behind the same `CredentialStore` contract
/// so a future tool that *does* use Credential Manager can be added here
/// without changing any tool adapter's code.
pub struct WindowsCredentialStore {
    known_paths: Vec<(&'static str, PathBuf)>,
}

impl WindowsCredentialStore {
    pub fn discover() -> Self {
        let mut known_paths = Vec::new();
        if let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            known_paths.push(("claude-code", home.join(".claude").join(".credentials.json")));
        }
        Self { known_paths }
    }
}

impl CredentialStore for WindowsCredentialStore {
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
            Err(e) => CredentialOutcome::StoreUnavailable { reason: e.to_string() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_key_is_not_found() {
        let store = WindowsCredentialStore { known_paths: vec![] };
        assert!(matches!(store.find("some-other-tool"), CredentialOutcome::NotFound));
    }

    #[test]
    fn missing_file_for_known_key_is_not_found_not_error() {
        let store = WindowsCredentialStore {
            known_paths: vec![("claude-code", PathBuf::from("Z:\\definitely\\does\\not\\exist.json"))],
        };
        assert!(matches!(store.find("claude-code"), CredentialOutcome::NotFound));
    }
}
