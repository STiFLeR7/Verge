# Verge Windows design system

The maintained specification is [DESIGN.md](../../DESIGN.md), updated for the 2026-09-08 creative-director review against the user's Image 1.

- [Central Windows tokens](../../platform/windows/src/tokens.rs): all dimensions are DIPs, converted at the raster boundary.
- [Native renderer](../../platform/windows/src/overlay.rs): shared silhouette, last-painted input geometry, background content polling, and one reversible morph.
- [Presentation](../../ui/ambient/windows/src/lib.rs): real usage and activity, relevant-tool filtering, four-tool maximum, and overflow.
- [Window contract](../../platform/windows/tests/window_contract.ps1): native hover, click-through, DPI, focus, and resource checks.
- [Logo provenance](../../THIRD_PARTY_NOTICES.md): existing Claude Code identity.

Historical frame-scale dimensions and earlier screenshots in the implementation reports describe previous passes. They are not the current geometry specification. Native fixtures remain synthetic renderer evidence, separate from live Claude data and real permission decisions.
