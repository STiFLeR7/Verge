/// What the ambient overlay should currently render. Deliberately tiny:
/// the overlay surface's job is display + hover + click, never business
/// logic (see docs/PRODUCT_ARCHITECTURE.md §13 on why `ui/ambient` is kept
/// this thin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayContent {
    pub lines: Vec<String>,
}

/// Create/run a borderless, transparent, always-on-top surface — or
/// honestly report it cannot. Returning "not possible here" is an expected,
/// non-error outcome for this contract, not a failure (see the capability
/// contract table in docs/PRODUCT_ARCHITECTURE.md §5).
///
/// This vertical slice's Windows implementation runs its own blocking
/// message loop, so `run` takes ownership and does not return until the
/// surface is closed; a future revision may need a non-blocking variant
/// once more than one platform implements this port.
pub trait OverlaySurface {
    fn run(self, content_source: impl Fn() -> OverlayContent + Send + 'static) -> std::io::Result<()>;
}
