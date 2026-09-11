---
name: Verge Windows Ambient Surface
description: A dense black instrument attached to the right display edge.
colors:
  material: "#030304"
  text-primary: "#ebebeb"
  text-secondary: "#9d9da0"
  working: "#d7dbe0"
  waiting: "#ebca5b"
  completed: "#2cd364"
  stopped: "#ff4c44"
  claude-identity: "#d97757"
---

# Verge ambient design system

Session-awareness addition: the account tooltip has a compact Sessions link
when verified Claude sessions exist. It replaces quota rows with one session's
name, model, explicitly labelled Context, activity and reported capacity.
Back/previous/next reuse the same lobe; permissions always override browsing.
Context never replaces the rail's account percentage or activity color.
See [the session specification](docs/design/SESSION_INTELLIGENCE.md).

Current font decision: the user selected **Inter**. Inter Regular and SemiBold
are embedded as private Windows font resources. Sizes and layout remain
unchanged. Windows font-selection tests verify Inter in both weights; the
system UI font is only a failure fallback. The SIL OFL license is included
with the font assets.

Updated 2026-09-08 against the user's target Image 1 and subsequent full-frame correct-size reference. Image 2 is the superseded implementation. Historical frame-scale measurements in earlier reports are not current specifications.

One dense black instrument grows from the right display edge. A persistent Claude mark and usage reading anchor the composition. One connected lobe opens leftward to disclose details. Rendering remains native Rust/Win32.

## 90% scale and Codex follow-up

The complete tooltip and rail now render at 0.9 of the base dimensions below, including type and input geometry. The connector is additionally shortened from 40 to 28 base DIP, with a 35% shallower root and a 1.5-DIP tip half-height. The popup body remains 280 base DIP (252 effective DIP); rail width is 75.6 effective DIP. Detail extension is now 308 base DIP; total expanded width is 352.8 effective DIP before physical rounding.

Codex now joins Claude using actual local rollout `token_count.rate_limits` records and recent task events. Usage remains an as-of reading and dims when stale; expired/invalid values disappear. No conversation content is shown. Working evidence expires after 120 seconds without new events; local rollout events are reported activity, not independently verified process liveness. Codex approval is not connected; Claude's permission flow remains unchanged. Claude and ChatGPT use their product marks; OpenCode uses its supplied SVG, centered with subpixel sampling.

Explicit --open reveals providers with reported usage or recent sessions for comparison. Normal launches retain active-first filtering. Codex metadata refreshes every three seconds, scanning the latest 32 rollout tails (512 KiB each); unavailable older tail data is not reconstructed or fabricated.

## Central vocabulary

[Windows tokens](platform/windows/src/tokens.rs) own geometry, typography, material, colors, state-light intensity, and motion. [The renderer](platform/windows/src/overlay.rs) shares silhouette calculations between painting and interaction. Tool identity comes from existing presentation data.

| Role | DIP | Weight |
|---|---:|---|
| Tool title | 24 | Regular |
| Usage percentage | 26 | Regular |
| Usage labels and readings | 17 | Regular |
| Sessions / status; resets / overflow | 15; 14 | Regular; rail counts semibold |
| Title rhythm / body line | 44 / 24 | — |

Use the privately loaded Inter Regular and SemiBold resources with grayscale GDI antialiasing and measured, centered numerals. Dimensions are logical units converted at window DPI. Labels use end ellipsis where necessary; percentages get a full 34-DIP line box. Explicit tabular shaping is not implemented.

| Element | DIP |
|---|---:|
| Rail width | 84 |
| Detail extension / total expanded width | 308 / 392 |
| Detail body / connector | 280 / 28 |
| Detail padding / usable width | 16 / 248 |
| Product mark (rail and header) | 28 |
| Usage ring / track / arc stroke | 52 / 7 / 4 |
| Ring-to-reading gap | 10 |
| Slot height / inter-slot gap | 96 / 24 |
| Rail shoulder / inverse edge flare | 40 / 44 |
| Detail corner | 26 |
| Usage row / bar thickness | 78 / 6 |
| Permission sheet / buttons | 260–332 / 36 rounded rectangle |
| Idle sliver | 5 × 80 |

The new size reference informs relative proportions: the ring is about three-fifths of the rail width; the detail body is about 3.3 rail widths. The illustration does not establish Windows DPI, so its bitmap pixels are not treated as logical units.

The detail lobe has rounded shoulders and a narrowing neck rooted at the selected identity. Inverse rail curves meet the display edge flush. At most four relevant tools appear; overflow is +N. Short logical displays reduce the visible count rather than shrinking typography or cutting off the rail. Primary-screen placement retains its upward bias where it fits and clamps the full surface inside the display.

## Material, identity, and state

Material is neutral near-black (3,3,4), fully opaque inside the antialiased silhouette. Desktop content cannot bleed through the reading. There is no backdrop blur, stroked material border, decorative gradient, neon outline, or cast shadow.

Claude retains its terracotta identity and [existing provenance](THIRD_PARTY_NOTICES.md). Rings show green completed, red stopped/error, yellow permission waiting, or an animated neutral arc while working. Neutral has no state light. Current identity and activity do not inherit usage age.

Usage remains independent of activity: green below 50%, yellow from 50% to below 70%, orange at 70% and above. Only tooltip bars encode usage; radial arcs encode activity. Both have smooth rounded endpoints. Missing denominators never produce invented percentages. Aged usage dims.

## Information and permission

The header gives the tool presence and shows a verified session count, including one session. Claude shows both 5-hour and weekly limits; missing windows say Not reported. Other providers show their most relevant reported quota. Codex windows retain explicit Codex labels within the ChatGPT identity. Reported state shares the percentage row; project/session metadata no longer competes with usage. Without quota windows, useful activity details remain available.

A real pending permission shows Claude, yellow attention, elapsed waiting time, concise action, working directory, action summary, Deny, and Approve or Approve…. The permission view replaces that agent’s usage view at the same top edge and rail anchor, growing downward for its summary and compact dark-tinted Deny / Approve controls with red / green labels. A new request selects its own agent. Permission labels use Inter Regular at 14 DIP, matching tooltip secondary text. Buttons are 36 DIP high with 8-DIP corners; action text is 17-DIP regular. Known JSON fields are summarized for display only. The original request remains available in the existing native full-action review. Summarized, clipped, or ambiguous content always requires that review before approval.

The real permission service remains authoritative. Request/session matching, pointer down/up identity matching, settled-geometry gating, the 650-ms arming delay, expiry, and explicit approval are retained. Dismissal never approves. Tab/Enter/Space/Escape and visible focus remain supported after the sheet receives keyboard focus.

Confirmed decisions suppress stale worker snapshots so a resolved request cannot reopen. The shared pipe reader now checks availability before reading: connected-but-empty pipes wait instead of prematurely denying. Disconnects and expiry still fail closed. A helper whose parent has exited can finish without panicking on a closed stdout pipe.

## Motion and input

One reversible 240-ms cubic ease-out drives material expansion and text reveal. Enter waits 100 ms; exit waits 220 ms. Switching identities closes the old lobe before reopening at the new identity. The logo remains stationary; the working arc rotates once per 1.6 seconds.

Live content polling runs on a background worker with a bounded channel. Slow disk/process/network reads do not block the native animation loop. Input uses the last successfully painted layout rather than a newer animation sample, preserving agreement with the actual window rectangle.

Usage fractions interpolate over the same 240-ms easing, preserving tool identity and never interpolating missing data from zero. Morphs finish at the exact target frame, and temporary corner radii are clamped to the growing surface. Morph and usage ticks use 16 ms; settled working ticks use 32 ms; other state-light ticks use 80 ms. Working light drifts 0.65 DIP over 12 seconds. Waiting gets one 600-ms response; completed light is static. The startup Windows reduced-motion setting disables motion. Dynamic WS_EX_TRANSPARENT, topmost, layered transparency, and no-activate behavior remain intact. SWP_SHOWWINDOW ensures a hidden console does not also hide the overlay.

## Verification and limits

`cargo fmt --check` and `cargo test --workspace` passed (58 tests). They cover domain/presentation behavior, permission lifecycle, native raster states, premultiplied alpha, stationary identity geometry, usage dimming, easing, overflow fit, and safe action summaries. VERGE_VISUAL_DIR exports native fixtures at 100%, 125%, 150%, and 200%.

The [visible-window contract](platform/windows/tests/window_contract.ps1) now expects a 75.6-DIP / 95-physical-pixel rail at 120 DPI. This pass's attempts stopped because the physical pointer moved during the check; the six-cycle live timing result is not claimed. Earlier geometry passed that contract, but that does not certify this revision. Native raster checks cover 100%, 125%, 150%, and 200%.

The separate permission_contract example passed approve, deny, dismiss, disconnect, and terminated-process cases through the actual native pipe/helper with isolated synthetic sessions, including an empty connection before the explicit decision. No real Claude action was approved during testing.

The [native permission UI contract](platform/windows/tests/permission_ui_contract.ps1) passed Deny, Escape dismissal, explicit approval through full review, and closing review without approval. It posts native keyboard/control messages to its own isolated fixture window and records decisions only for its synthetic request. Actual mouse targeting during pointer movement is not certified by this check.

Current evidence: [actual application, real Claude data](docs/design/evidence/final-actual-125.png), [synthetic usage](docs/design/evidence/final-usage-125.png), [synthetic permission](docs/design/evidence/final-permission-125.png), [synthetic working](docs/design/evidence/final-working-125.png), [synthetic overflow](docs/design/evidence/final-overflow-125.png). The actual application was rebuilt and opened with real 71% usage and one idle session. It remains available for review with --open.

Primary-screen placement remains the supported model. Multi-monitor relocation, live reduced-motion preference changes, and a native UI Automation tree remain outside this pass. Screen-reader support is not claimed.

Explicit opening: run `verge.exe --open` to reveal real Claude data even while idle. The initial detail view stays open until the pointer enters and leaves it; hover continues to work afterward. Ordinary launches retain ambient filtering.

## Provider discovery and idle

The existing background worker discovers installed tools every 30 seconds. See [provider support and sources](docs/design/PROVIDER_DISCOVERY.md). Normal idle collapses to the existing vertical edge bar, white in dark theme and black in light theme. Explicit --open reveals discovered providers for inspection. Account brands are grouped; unrelated quota products are never added together.

## Inactivity and secondary readings

After 30 seconds without pointer interaction on Verge or permission keyboard/button input, the surface collapses to its theme-aware idle bar even while an agent works. Hovering the bar reveals it. Pending permission requests or reported waiting-for-user states override the timeout. Background usage refreshes do not reset it. Weekly tooltip labels and readings use the existing 14-DIP Inter size; the main notch reading remains unchanged. Every positive reported session count has a dark capsule and neutral outline, capped visually at 99+.

## Verified typography and distribution

The user selected Inter from Google Fonts after reviewing the target image.
Windows embeds `Inter-Regular.ttf` and `Inter-SemiBold.ttf` and registers them
privately for the Verge process before creating GDI fonts. macOS packages the
same files and registers them through CoreText. Native rasterization differs
by platform, so the same family and measurements do not imply pixel-identical
antialiasing. Inter remains licensed under the SIL Open Font License included
with the assets; OpenAI Sans is not bundled or substituted.
