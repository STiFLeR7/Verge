/// Restrained state tint per `docs/design/VERGE_AMBIENT_DESIGN.md` §7/§8.
/// `Neutral` is used whenever no real activity signal exists for a tool yet
/// — see the "Activity Gap" section of
/// `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`. Collapsing
/// thinking/researching/implementing/executing into one `Working` value is
/// deliberate (design spec §7), not a simplification made here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTint {
    Neutral,
    Working,
    Waiting,
    Completed,
}

/// One tool's identity plus presentation-ready state, exactly as the
/// ambient surface should render it — composed by `ui/ambient` from
/// `AmbientState`, never containing implementation vocabulary (no
/// "adapter", "provider", "local state", "capability", ... — design spec
/// §21). The platform surface only draws what it is given; it never
/// interprets `Availability`, `Fidelity`, or `Recency` itself.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolGlyph {
    /// Short human name, e.g. "Claude".
    pub label: String,
    /// A single-character identity mark standing in for the tool's real
    /// brand mark until vector/bitmap logo assets are integrated — see
    /// `docs/design/VERGE_AMBIENT_IMPLEMENTATION.md`.
    pub mark: char,
    /// The tool's own brand color, never remapped by Verge (design spec §13).
    pub brand_color: (u8, u8, u8),
    pub state: StateTint,
    /// Whether to draw the static "there is a metric here" identity ring.
    /// Never a percentage fill unless a real `Fraction` reading exists —
    /// design spec §9's explicit rule against inventing a denominator.
    pub has_metric: bool,
    /// True if the underlying reading is `Recency::Aged` and should render
    /// visually dimmed — a material treatment, never additional text
    /// (design spec §9's "appropriately dimmed" instruction).
    pub dimmed: bool,
    /// Plain, human, jargon-free lines shown only in the expanded view.
    pub detail_lines: Vec<String>,
}

/// What the ambient overlay should currently render. Deliberately tiny and
/// data-only: the overlay surface's job is display + hover + click, never
/// business logic (design spec's whole premise, and
/// `docs/PRODUCT_ARCHITECTURE.md` §13 on why `ui/ambient` is kept this
/// thin).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OverlayContent {
    pub glyphs: Vec<ToolGlyph>,
    /// Set when more tools are active than the surface's density cap
    /// allows — rendered as a compact "+N" mark, never a longer list
    /// (design spec §18). `ui/ambient` computes this; the platform surface
    /// only draws it.
    pub overflow_count: Option<u32>,
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
    fn run(
        self,
        content_source: impl Fn() -> OverlayContent + Send + 'static,
    ) -> std::io::Result<()>;
}
