# Native portability foundation — 2026-09-10

Windows remains the reference native implementation. Linux now meets the automated X11/XWayland surface contract; macOS still needs a real-host interaction pass.

| Capability | Windows | Linux | macOS |
|---|---|---|---|
| Shared Rust model and presentation | Yes | Yes | Yes, via versioned local JSON helper |
| Local Codex session/context/limits | Yes | Wired | Wired |
| Claude local usage metadata | Yes | Wired when metadata exists | Wired when metadata exists |
| Verified Claude live process and approval broker | Yes | Not implemented | Not implemented |
| Native surface | Win32 | X11/XWayland | AppKit panel baseline |
| Session navigation | Yes | Provider tabs, Back, previous, next | Back, previous, next |
| 30-second inactivity collapse | Yes | Yes; waiting holds open | Implemented, runtime verification pending |
| Inter and final visual treatment | Yes | Bundled Inter; native contract passes | Bundled Inter, baseline layout only |

## Builds

Run `powershell -File scripts/build-portable.ps1` on Windows, or `bash scripts/build-portable.sh` on Linux/macOS. Outputs are under `dist`. Builds target the current host architecture. Windows output includes the approval helper but does not install hooks or move the existing configured helper. Linux needs DISPLAY and an X11 server (including XWayland); native Wayland is not supported. macOS requires Xcode command-line tools and Rust; the result is an ad-hoc-signed Verge.app for macOS 13+. Developer ID signing/notarization and public distribution are not included.

The AppKit shell runs the sibling `verge-state` helper every three seconds. The helper reads existing local metadata through Rust adapters and returns presentation data only. It does not install integrations, send prompts, or decide permissions. Failed reads show unavailable rather than retaining stale content. Claude signal collection on Unix still requires a future supported installer/process probe. The legacy Windows signal installer must not be presented as a Unix installer.

## Reference and verification

`D:/codenotch` was inspected for its native nonactivating NSPanel approach (status-bar level, all Spaces, visible-frame positioning). The Swift shell is a separate minimal implementation; Codenotch provider, updater, and credential code was not copied.

Local Windows workspace tests and Rust all-target checks for x86_64 Linux and Apple Silicon macOS pass. The native Linux contract runs the real X11 surface in nested Xephyr/Xvfb and verifies bundled Inter rendering, theme contrast, navigation, inactivity collapse, hover reveal, focus retention, EWMH state, input shape, work-area anchoring, and RandR resize. The macOS CI job compiles Swift, decodes a live `verge-state` snapshot through the AppKit executable, runs the native panel contract, ad-hoc signs the app, and creates its ZIP. A real-host macOS interaction pass is still required.

Next: complete the real-host macOS interaction pass and the documented release verification gates. Unix approval controls remain outside v1.0.0.

## Native Linux follow-up

Provider selection and session IDs survive source reordering; Back returns to overview and the session count has no click action. A bounded worker channel keeps metadata reads off the X11 event loop. Waiting observations hold the panel open without enabling approval decisions. The macOS bridge now exports transient hashed view IDs (not authorization tokens) so its selected session also survives reordering; the counter uses Inter and white text.

Run `xvfb-run -a -s "-screen 0 1280x900x24" platform/linux/x11/tests/run_native_smoke.sh target/release/verge` after building on Linux. The runner starts nested Xephyr so the test can exercise a real RandR mode change. It creates isolated temporary Codex session metadata and never reads real accounts.

Remaining: Linux keyboard/accessibility review, macOS native panel interaction and display-change verification, and Unix discovery infrastructure. Direct approval remains intentionally limited to Claude on Windows for v1.0.0.
