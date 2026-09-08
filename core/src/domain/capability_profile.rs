use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    AmbientOverlay,
    SystemTray,
    SecureCredentialStore,
    LoginAtStartup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityLevel {
    Full,
    Reduced,
    None { reason: String },
}

/// What this OS/session/desktop environment can currently do for us.
/// Produced by the platform layer, consumed by the application layer to
/// choose a Product Mode — never an ad hoc `if cfg!(windows)` scattered
/// through application logic.
#[derive(Debug, Clone, Default)]
pub struct CapabilityProfile {
    levels: HashMap<Capability, CapabilityLevel>,
}

impl CapabilityProfile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, capability: Capability, level: CapabilityLevel) {
        self.levels.insert(capability, level);
    }

    /// Capabilities not reported at all are `None` with an explicit
    /// "not reported" reason — never silently treated as `Full`.
    pub fn level(&self, capability: Capability) -> CapabilityLevel {
        self.levels
            .get(&capability)
            .cloned()
            .unwrap_or(CapabilityLevel::None {
                reason: "capability not reported by this platform build".to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreported_capability_is_none_not_full() {
        let profile = CapabilityProfile::new();
        assert_eq!(
            profile.level(Capability::AmbientOverlay),
            CapabilityLevel::None {
                reason: "capability not reported by this platform build".to_string()
            }
        );
    }

    #[test]
    fn set_capability_is_returned() {
        let mut profile = CapabilityProfile::new();
        profile.set(Capability::AmbientOverlay, CapabilityLevel::Full);
        assert_eq!(
            profile.level(Capability::AmbientOverlay),
            CapabilityLevel::Full
        );
    }
}
