# Capability tests

Native behavior is covered by platform contracts rather than unit tests. The
matrix below records current evidence against the eight checks from the
original overlay-capability spike. `PASS` means an automated repository
contract exercises the behavior; `MANUAL` means source or historical evidence
exists but the behavior is not yet protected by an automated contract;
`UNSUPPORTED` means the current platform tier does not provide it.

| Capability | Windows | Linux X11/XWayland | macOS AppKit |
|---|---|---|---|
| Borderless native surface | PASS — popup with no caption/frame | PASS — override-redirect window | PASS — borderless nonactivating panel |
| Always on top over normal windows | PASS — topmost style and focus retention | PASS — `_NET_WM_STATE_ABOVE` contract | PASS — status-bar panel level |
| Always on top with fullscreen content | PASS — real fullscreen fixture remains below Verge | MANUAL | MANUAL — auxiliary flag passes; behavioral evidence pending |
| Click-through outside visible material | PASS — dynamic `WS_EX_TRANSPARENT` assertion | PASS — expanded and collapsed input shapes | MANUAL — native hit-region coverage pending |
| Interactive visible region | PASS — physical hover and native session/permission contracts | PASS — native mouse navigation under Xephyr | MANUAL — build-host snapshot only |
| Multi-monitor placement | MANUAL | MANUAL | MANUAL |
| Display/DPI/work-area changes | PASS — monitor work-area, `WM_DISPLAYCHANGE` and `WM_DPICHANGED` recovery | PASS — real RandR mode change in Xephyr | PASS — visible-frame geometry and screen-change notification contract |
| Session/window lifecycle | PASS — fixture creation, interaction and cleanup | PASS — process start, interaction and termination under Xephyr | PASS — panel creation and termination contract |

## Commands

Windows:

```powershell
pwsh -NoProfile -File platform/windows/tests/window_contract.ps1
pwsh -NoProfile -File platform/windows/tests/session_ui_contract.ps1 -Brand Claude
pwsh -NoProfile -File platform/windows/tests/session_ui_contract.ps1 -Brand ChatGPT
pwsh -NoProfile -File platform/windows/tests/permission_ui_contract.ps1
```

Linux after building the release executable and installing Xvfb, Xephyr,
`xdotool`, `x11-utils`, and `x11-xserver-utils`:

```sh
xvfb-run -a bash platform/linux/x11/tests/run_native_smoke.sh target/release/verge
```

macOS runs the build-host bridge check inside `scripts/build-portable.sh` and
the native panel contract with:

```sh
swift test --package-path ui/ambient/macos
```

The executable contract is
[`ui/ambient/macos/Tests/native_contract.swift`](../../ui/ambient/macos/Tests/native_contract.swift).
The remaining `MANUAL` cells require real multi-display or interaction
evidence before v1.0.0.

Reliability runs use `scripts/soak.ps1` on Windows and `scripts/soak.sh` on Linux/macOS. Long-duration results are tracked in the [v1 verification ledger](../../docs/releases/V1_VERIFICATION.md); the presence of a collector does not count as a completed soak.

Current evidence and its limits are recorded in
[`docs/design/E2E_PORTABILITY_2026-09-10.md`](../../docs/design/E2E_PORTABILITY_2026-09-10.md).
