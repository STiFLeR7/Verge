# Capability tests

Native behavior is covered by platform contracts rather than unit tests. The
matrix below records current evidence against the eight checks from the
original overlay-capability spike. `PASS` means an automated repository
contract exercises the behavior; `MANUAL` means source or historical evidence
exists but the behavior is not yet protected by an automated contract;
`UNSUPPORTED` means the current platform tier does not provide it.

| Capability | Windows | Linux X11/XWayland | macOS AppKit |
|---|---|---|---|
| Borderless native surface | MANUAL — layered tool-window style passes; popup/frame assertion pending | MANUAL — override-redirect X11 window; contract assertion pending | MANUAL — Swift build passes; panel contract pending |
| Always on top over normal windows | PASS — topmost style and focus retention | MANUAL — `_NET_WM_STATE_ABOVE`; behavioral assertion pending | MANUAL — status-bar panel level; launch assertion pending |
| Always on top with fullscreen content | MANUAL | MANUAL | MANUAL |
| Click-through outside visible material | PASS — dynamic `WS_EX_TRANSPARENT` assertion | MANUAL — X11 input-shape coverage pending | MANUAL — native hit-region coverage pending |
| Interactive visible region | PASS — physical hover and native session/permission contracts | PASS — native mouse navigation under Xvfb | MANUAL — build-host snapshot only |
| Multi-monitor placement | MANUAL | MANUAL | MANUAL |
| Display/DPI/work-area changes | MANUAL — single-window DPI is covered | MANUAL | MANUAL |
| Session/window lifecycle | PASS — fixture creation, interaction and cleanup | PASS — process start, interaction and termination under Xvfb | MANUAL — build/package lifecycle only |

## Commands

Windows:

```powershell
pwsh -NoProfile -File platform/windows/tests/window_contract.ps1
pwsh -NoProfile -File platform/windows/tests/session_ui_contract.ps1 -Brand Claude
pwsh -NoProfile -File platform/windows/tests/session_ui_contract.ps1 -Brand ChatGPT
pwsh -NoProfile -File platform/windows/tests/permission_ui_contract.ps1
```

Linux after building the release executable:

```sh
xvfb-run -a python3 platform/linux/x11/tests/native_smoke.py dist/linux/verge
```

macOS currently runs the build-host bridge check inside
`scripts/build-portable.sh`. It compiles and executes the Swift binary without
opening the panel. Phase 3 of the
[v1 release plan](../../docs/superpowers/plans/2026-09-10-verge-v1.0.0-release.md)
adds the missing native panel contract and completes this matrix.

Current evidence and its limits are recorded in
[`docs/design/E2E_PORTABILITY_2026-09-10.md`](../../docs/design/E2E_PORTABILITY_2026-09-10.md).
