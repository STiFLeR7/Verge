# Troubleshooting

## Verge appears to be missing

On Windows, run `verge.exe --open` to request the expanded view. After 30 seconds without interaction it can collapse to a narrow edge bar even when an agent is working. Hover the bar to reveal it. Check Task Manager before launching another copy.

On Linux, confirm `DISPLAY` is set and an X11/XWayland server is reachable. Native Wayland is not implemented. The current Linux surface is a simpler text panel with a thin idle bar.

On macOS, launch the packaged `Verge.app`, which includes the Rust reader. Running a standalone Swift executable without its sibling `verge-state` produces an unavailable-reader message. Native Mac verification is still pending.

## Only one agent appears

An installed application is not necessarily active or reporting data. Windows discovery and the normal surface prioritize meaningful signals. Use `--open` on Windows to inspect available data, then check the source-specific [provider notes](design/PROVIDER_DISCOVERY.md).

The ChatGPT label represents the local Codex integration; it does not include arbitrary browser or cloud conversations. Linux/macOS have less discovery support than Windows.

## A usage window or context value is missing

Verge displays fields reported by the source. Claude's five-hour/weekly limits depend on valid status-line metadata. Codex limits and context depend on recent local rollout records. Missing capacity, stale observations and expired windows must not become invented percentages.

Codex context is derived from the last reported request's total tokens and context capacity, not cumulative session token totals. Working evidence expires if fresh events stop. See [session intelligence](design/SESSION_INTELLIGENCE.md).

## Claude permission requests are denied

If the Verge hook is installed, first start Verge from the same folder as the registered `verge-claude-hook.exe`. Keep both files together. Reload Claude after changing its hook registration. Moving the portable directory does not migrate the absolute path stored in Claude settings.

The running helper denies when the broker is absent, disconnected, timed out or fails identity validation. To restore Claude-managed decisions, run `scripts/install-claude-signals.ps1 -Mode Uninstall` and reload Claude. The removal preserves unrelated hooks and later status-line changes; it does not overwrite newer settings from an old backup. Follow the [full recovery guide](design/CLAUDE_APPROVAL_SETUP.md).

If installation reports an unsupported status-line setup or a different registered helper, inspect the existing configuration instead of forcing the installer past its guard.

## Native Windows tests fail on pointer position

The hover test uses the real pointer. Close the normal overlay, run fixtures sequentially and leave the pointer still during the repeated hover section. Keep the desktop unlocked. The targeted `-InactivityOnly` mode tests the timer separately; it does not replace the normal hover assertions.

See [CONTRIBUTING.md](../CONTRIBUTING.md) for commands. Test fixtures temporarily replace the live overlay, so an installed Claude gate can be unavailable during testing.

## Reporting a problem

Use the repository's bug template. Include revision/build, OS, display scale/backend, minimal steps, expected behavior and a sanitized screenshot or error. Identify whether data is real or synthetic. Do not upload `.credentials.json`, raw rollout files, private project paths or permission commands. Use [SECURITY.md](../SECURITY.md) for sensitive reports.
