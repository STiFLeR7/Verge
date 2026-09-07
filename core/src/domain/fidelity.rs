/// How a number was derived. Independent of how current it is (`Recency`) —
/// see docs/PRODUCT_ARCHITECTURE.md §6. Never collapse these two axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fidelity {
    /// The vendor's own endpoint or local state, unambiguous.
    Official,
    /// We computed it ourselves from a raw signal (e.g. counting local
    /// activity), rather than reading a vendor-published number.
    Derived,
    /// A heuristic or declared ceiling, ours or the user's.
    Estimated,
}
