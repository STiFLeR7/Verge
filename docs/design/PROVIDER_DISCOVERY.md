# Provider discovery and observation

Updated 2026-09-08. Verge reuses its existing background worker, scanning every 30 seconds. A separate Windows service is unnecessary: discovery and event files belong to the signed-in user.

| Provider | Detection | Activity / quota |
|---|---|---|
| Claude / Claude Code | CLI, app, running process | Existing real hooks and reported 5-hour / weekly statusline windows |
| ChatGPT / Codex | CLI, app, running process | Local Codex rollout events and Codex quota windows |
| OpenCode, Grok, Cursor, Antigravity | Known commands, apps or processes | Discovery only; no invented usage or activity |
| Pi coding agent | CLI | Installed extension observer; event contract tested locally |
| KiloCode | CLI / VS Code extension | Installed plugin observer; event contract tested locally |
| Hermes | CLI | Installed plugin observer; requires `hermes plugins enable verge-observer` |

Discovery checks PATH, common user installation locations, running executable names and Windows app registrations. Stale npm launcher scripts whose package targets are missing are rejected. Custom locations and browser-only installations can be missed. Detection establishes presence, not authentication or working status.

Observers write only provider, hashed session identity, activity state and timestamp. They never approve actions or store prompts. Existing observer files are not overwritten. Working/waiting expires to idle after 120 seconds without evidence; records older than ten minutes are ignored. Pi, Kilo and Hermes event contracts pass isolated tests, but their actual hosts were not available for end-to-end validation on this machine. Grok has no verified local lifecycle integration here.

Claude's current input provides weekly usage but omits its 5-hour reading; the UI says **Not reported** until the source provides it. Grouping ChatGPT/Codex uses one brand identity while preserving Codex-specific quota names; it does not sum unrelated limits or establish account identity. Direct permission decisions currently remain connected only to Claude's real request service.

Primary references:

- [Claude statusline quota fields](https://code.claude.com/docs/en/statusline) and [hooks, including StopFailure](https://code.claude.com/docs/en/hooks).
- [Codex app-server account and rate-limit interface](https://learn.chatgpt.com/docs/app-server).
- [Kilo plugin events and discovery paths](https://kilo.ai/docs/automate/extending/plugins).
- [Pi extension event contract](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md).
- [Hermes hooks](https://hermes-agent.nousresearch.com/docs/user-guide/features/hooks/) and [plugin enabling](https://hermes-agent.nousresearch.com/docs/user-guide/features/plugins/).
- [Grok product overview](https://docs.x.ai/grok/overview).
