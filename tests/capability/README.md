# Capability tests

Real OS/compositor integration — window creation, transparency, click-through,
always-on-top — cannot be meaningfully unit-tested (see
`docs/PRODUCT_ARCHITECTURE.md` §19). This category is manual/integration,
derived directly from the disposable overlay-capability spike's methodology
(`D:\overlay-capability-spike\SPIKE_RESULTS.md`), not an automated suite.

## Status

No automated harness exists yet. The only capability verification performed
so far is the manual bring-up run recorded in `docs/STATUS.md` and
`docs/design/evidence/vertical_slice_overlay_crop.png` — a real, live run of
`verge-desktop` against this machine's actual Claude Code installation,
screenshotted as a tight crop around the overlay only (see that file's
directory for the same privacy discipline established by the spike: no
full-desktop captures).

## What a future automated pass here should do

Re-run the eight capability tests from `SPIKE_RESULTS.md` §9 (borderless,
always-on-top-normal, always-on-top-fullscreen, click-through, interactive
region, multi-monitor, display changes, session lifecycle) against each
`platform/*` implementation as it's built, kept alive as a living regression
suite rather than a one-time spike, exactly as
`docs/PRODUCT_ARCHITECTURE.md` §19 specifies.
