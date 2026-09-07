/// Whether a reading exists at all right now, and why not if it doesn't.
/// Never invent a value to fill the gap when this is not `Available` — see
/// docs/PRODUCT_ARCHITECTURE.md §6 and the "never invent usage" invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    Available,
    /// Sign in via the owning tool.
    Unauthenticated,
    /// Credential exists, OS/user declined access. Distinct from
    /// `Unauthenticated` on purpose.
    AccessDenied,
    /// The account genuinely has nothing to meter.
    NotMetered,
    /// This platform, desktop environment, or tool version cannot provide
    /// this signal at all. `Unsupported` must never be reasoned about as if
    /// it meant `broken`.
    Unsupported { reason: String },
    /// Network/rate-limited; retry scheduled.
    Unreachable,
    /// Unexpected. Carries an internal diagnostic never shown raw to the
    /// user.
    Error { diagnostic: String },
}
