# Claude Code — Windows local state (empirical discovery)

Established by direct inspection of a real, in-use Claude Code installation
on the development machine (Windows 10, build 10.0.26200) on 2026-09-07 —
not assumed from `codenotch`'s macOS-only reference. Only key names and
structural shapes are recorded here; no secret values or user content are
reproduced anywhere in this repository or its history.

## Credential

Path: `%USERPROFILE%\.claude\.credentials.json`

```json
{
  "claudeAiOauth": {
    "accessToken": "...",
    "refreshToken": "...",
    "expiresAt": 0,
    "refreshTokenExpiresAt": 0,
    "scopes": [],
    "subscriptionType": "...",
    "rateLimitTier": "..."
  },
  "mcpOAuth": { }
}
```

**Finding worth flagging explicitly:** this is a plaintext JSON file, not an
entry in Windows Credential Manager. `docs/PRODUCT_ARCHITECTURE.md`'s
`CredentialStore` contract describes "read a named secret from the OS's
secure store" in a way that reads as Credential-Manager-shaped; for this
tool, on this OS, there is nothing in Credential Manager to read. The
`WindowsCredentialStore` implementation in `platform/windows` is honest
about this: it is a per-known-key file lookup, not a Credential Manager
wrapper, kept behind the same `CredentialStore` trait so a future tool that
*does* use Credential Manager can be added without touching any tool
adapter. This is not a violation of the architecture's intent (never
persist a credential ourselves, never write one, never log one) — only a
correction to which OS mechanism a given vendor actually uses.

The vertical slice's `ClaudeCodeUsageSource` never calls `SecretHandle::expose()`
on this file's contents — it only uses the store's `Found`/`NotFound`/
`AccessDenied` outcome to decide `Availability`. Reading the OAuth token's
actual value is only necessary for a future `UsageSource` that calls
Anthropic's own usage endpoint (see "What's deliberately not built yet"
below).

## Local usage/activity stats

Path: `%USERPROFILE%\.claude\stats-cache.json`

Top-level keys observed: `version`, `lastComputedDate`, `dailyActivity`,
`dailyModelTokens`, `dailyModelTokensVersion`, `modelUsage`, `totalSessions`,
`totalMessages`, `longestSession`, `firstSessionDate`, `hourCounts`.

`dailyActivity` is a list, most-recent-last, of:

```json
{ "date": "2026-03-05", "messageCount": 2432, "sessionCount": 14, "toolCallCount": 350 }
```

**Finding worth flagging explicitly:** `lastComputedDate` was observed
**weeks behind** the file's own filesystem mtime and behind the actual
current date on this machine. This cache is not recomputed on every Claude
Code session. Consequently `ClaudeCodeUsageSource` deliberately reports the
count against the *actual date the freshest entry is for* (e.g. "messages
on 2026-08-20"), never as "today" — labeling a three-week-old entry as
"today" would be exactly the kind of silent recency/fidelity conflation
`docs/PRODUCT_ARCHITECTURE.md` §6 exists to prevent.

## Why Claude Code, not Cursor or Codex, for this vertical slice

All three Tier-1 candidates were checked on this machine before picking one:

- **Cursor:** only `%APPDATA%\Cursor\auth.json` exists — no
  `User\globalStorage` directory, no evidence of a real, current Cursor
  installation using this machine's profile. Not viable to build an
  empirically-grounded adapter against right now.
- **Codex:** genuinely, richly present (`config.toml`, `auth.json`, a
  `goals_*.sqlite` WAL database, session archives, browser/computer-use
  subdirectories) — a real candidate, deliberately not chosen for this pass
  simply because a one-tool vertical slice needs one tool, and Claude Code's
  state is the cleanest single-file, single-mechanism signal of the three.
  Codex is a strong candidate for the next tool adapter.
- **Claude Code:** clean, well-understood, single-file credential + a
  separate single-file local stats cache. Chosen for this reason, not
  because the other two are unviable long-term.

## What's deliberately not built yet

- Reading Anthropic's official usage/rate-limit endpoint (would need the
  actual `accessToken` value, an HTTP client, and real network I/O — out of
  scope for a network-free first vertical slice). This is the `Official`-
  fidelity path referenced in the architecture doc's Tier-1 reasoning for
  Claude Code; today's adapter only reaches `Derived` fidelity, from local
  state alone.
- Any `ActivitySource` implementation for Claude Code (session busy/waiting/
  idle detection). `daemon.status.json` (`supervisorPid`, `workers`) is a
  plausible future signal, not yet investigated.
- Discovery of more than one account/profile.
