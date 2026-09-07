use crate::domain::{Capability, CapabilityLevel, CapabilityProfile};

/// See docs/PRODUCT_ARCHITECTURE.md §15. `DIAGNOSTIC` (debug/service only,
/// never a shipped default) is intentionally not modeled yet — nothing in
/// the current vertical slice produces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductMode {
    /// Ambient overlay present, all connected accounts reporting normally.
    Full,
    /// Some capability is degraded but a persistent ambient presence still
    /// exists (e.g. tray icon instead of edge overlay).
    Reduced,
    /// No persistent ambient surface at all; the same data is available by
    /// deliberately opening the detail window.
    OnDemand,
}

/// The single source of truth for choosing a mode is the active
/// `CapabilityProfile` — never an ad hoc `if cfg!(windows)` scattered
/// through the UI (see docs/PRODUCT_ARCHITECTURE.md §15).
pub fn select_product_mode(profile: &CapabilityProfile) -> ProductMode {
    match profile.level(Capability::AmbientOverlay) {
        CapabilityLevel::Full => ProductMode::Full,
        CapabilityLevel::Reduced => ProductMode::Reduced,
        CapabilityLevel::None { .. } => ProductMode::OnDemand,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_overlay_capability_yields_full_mode() {
        let mut profile = CapabilityProfile::new();
        profile.set(Capability::AmbientOverlay, CapabilityLevel::Full);
        assert_eq!(select_product_mode(&profile), ProductMode::Full);
    }

    #[test]
    fn missing_overlay_capability_yields_on_demand_not_full() {
        let profile = CapabilityProfile::new();
        assert_eq!(select_product_mode(&profile), ProductMode::OnDemand);
    }

    #[test]
    fn reduced_overlay_capability_yields_reduced_mode() {
        let mut profile = CapabilityProfile::new();
        profile.set(Capability::AmbientOverlay, CapabilityLevel::Reduced);
        assert_eq!(select_product_mode(&profile), ProductMode::Reduced);
    }
}
