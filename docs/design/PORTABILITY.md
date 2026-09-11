# Native portability foundation — 2026-09-10

Windows remains the complete native implementation. This change starts native Linux and macOS support; it does not claim equal feature or visual completeness.

| Capability | Windows | Linux | macOS |
|---|---|---|---|
| Shared Rust model and presentation | Yes | Yes | Yes, via versioned local JSON helper |
| Local Codex session/context/limits | Yes | Wired | Wired |
| Claude local usage metadata | Yes | Wired when metadata exists | Wired when metadata exists |
| Verified Claude live process and approval broker | Yes | Not implemented | Not implemented |
| Native surface | Win32 | X11/XWayland text baseline | AppKit panel baseline |
| Session navigation | Yes | Provider tabs, Back, previous, next | Back, previous, next |
| 30-second inactivity collapse | Yes | Yes; waiting holds open | Implemented, runtime verification pending |
| Inter and final visual treatment | Yes | Legacy X11 core font; pending | Bundled Inter, baseline layout only |

## Builds

Run `powershell -File scripts/build-portable.ps1` on Windows, or `bash scripts/build-portable.sh` on Linux/macOS. Outputs are under `dist`. Builds target the current host architecture. Windows output includes the approval helper but does not install hooks or move the existing configured helper. Linux needs DISPLAY and an X11 server (including XWayland); native Wayland is not supported. macOS requires Xcode command-line tools and Rust; the result is an ad-hoc-signed Verge.app for macOS 13+. Developer ID signing/notarization and public distribution are not included.

The AppKit shell runs the sibling `verge-state` helper every three seconds. The helper reads existing local metadata through Rust adapters and returns presentation data only. It does not install integrations, send prompts, or decide permissions. Failed reads show unavailable rather than retaining stale content. Claude signal collection on Unix still requires a future supported installer/process probe. The legacy Windows signal installer must not be presented as a Unix installer.

## Reference and verification

`D:/codenotch` was inspected for its native nonactivating NSPanel approach (status-bar level, all Spaces, visible-frame positioning). The Swift shell is a separate minimal implementation; Codenotch provider, updater, and credential code was not copied.

Local Windows workspace tests and Rust all-target checks for x86_64 Linux and Apple Silicon macOS pass. Native Linux workspace tests also pass in Ubuntu 24.04 under WSL; the real X11 surface passed collapse, hover reveal, and focus-retention checks under Xvfb. GitHub Actions run `34474551752` passed on Windows, Ubuntu, and macOS: the macOS job compiled Swift, decoded a live `verge-state` snapshot through the AppKit executable, ad-hoc signed the app, and created its ZIP. That build-host contract does not launch or interact with the panel. Full Linux visual/accessibility review and native macOS UI tests remain required before either reaches its v1 support tier.

Next: validate the native builds on their hosts, then bring Linux rendering/navigation and macOS visuals to the established design; port provider discovery and verified process observation before adding any Unix approval controls.

## Native Linux follow-up

Provider selection and session IDs survive source reordering; Back returns to overview and the session count has no click action. A bounded worker channel keeps metadata reads off the X11 event loop. Waiting observations hold the panel open without enabling approval decisions. The macOS bridge now exports transient hashed view IDs (not authorization tokens) so its selected session also survives reordering; the counter uses Inter and white text.

Run `xvfb-run -a python3 platform/linux/x11/tests/native_smoke.py target/release/verge` after building on Linux. The smoke test creates isolated temporary Codex session metadata and never reads real accounts. On WSL, where the filesystem X11 socket may be unavailable, use `VERGE_TEST_TCP=1 xvfb-run -a -l --server-args="-screen 0 1280x1024x24 -listen tcp" python3 platform/linux/x11/tests/native_smoke.py target/release/verge`. This uses Xvfb's temporary authentication, not an unauthenticated user display.

Remaining: Linux Inter/vector rendering, theme-aware idle color, monitor/work-area tracking and keyboard/accessibility navigation; macOS native panel interaction and display-change verification; Unix discovery infrastructure. Linux currently uses a white idle bar and legacy core-font text. Direct approval remains intentionally limited to Claude on Windows for v1.0.0.
