# Codex — Linux local state (empirical discovery)

Established 2026-09-07 by installing a fresh, standard copy of OpenAI's
Codex CLI inside a disposable WSL2 Ubuntu 26.04 environment (`Ubuntu`
distro) and cross-referencing its behavior against its own public source
(`github.com/openai/codex`, Apache-2.0, tag `rust-v0.153.4`). No real
OpenAI/ChatGPT account was ever signed in during this investigation — every
claim below is backed by either the tool's own `doctor` diagnostic output
against a genuinely unauthenticated install, or by reading the actual Rust
source that implements the behavior, not by analogy to the Windows Codex
installation observed in a prior, unrelated session (that installation may
include local extensions/plugins beyond the base product and was
deliberately not used as a model here — see
`docs/design/claude-code-windows-local-state.md`'s own equivalent caveat for
Cursor).

## Environment

- Distro: Ubuntu 26.04 LTS, inside WSL2 (`Ubuntu` distro)
- Desktop: none (headless shell; the ambient-surface work in this slice used
  a separate nested X11 session — see `docs/design/SECOND_VERTICAL_SLICE.md`)
- Display server: N/A for this discovery phase
- Codex version: `0.153.4` (`codex-cli 0.153.4`, tag `rust-v0.153.4`)
- Installation method: direct download of the official
  `codex-x86_64-unknown-linux-musl.tar.gz` release asset from
  `github.com/openai/codex/releases`, extracted to a plain directory and run
  directly (no npm, no Homebrew, no installer script executed — the CLI is
  a single static Rust/musl binary with **no Node.js dependency** on this
  path, confirmed by successfully running it with zero JS runtime present in
  the environment)

## Authentication storage

**CONFIRMED**, by direct source inspection of `codex-rs/login/src/auth/storage.rs`
and `codex-rs/utils/home-dir/src/lib.rs`:

- Home directory: `$CODEX_HOME` if set (must already exist as a directory),
  else `dirs::home_dir()/.codex` — i.e. `$HOME/.codex` on Linux. This is the
  exact same `~/.codex` convention already observed on the Windows host in a
  prior session; the *mechanism* determining it (an env var override falling
  back to a portable home-dir crate) is now confirmed from source, not
  assumed from the coincidence of the two paths matching.
- Credential file: `$CODEX_HOME/auth.json`, a plaintext JSON file, matching
  struct `AuthDotJson`:

  ```json
  {
    "auth_mode": "...",
    "OPENAI_API_KEY": null,
    "tokens": {
      "id_token": "<JWT>",
      "access_token": "<JWT>",
      "refresh_token": "...",
      "account_id": "..."
    },
    "last_refresh": "2026-09-07T00:00:00Z",
    "agent_identity": null,
    "personal_access_token": null,
    "bedrock_api_key": null,
    "bedrock_access_keys": null
  }
  ```

  (Field names and shape taken directly from `AuthDotJson` in
  `codex-rs/login/src/auth/storage.rs` and the crate's own
  `write_chatgpt_auth_json` test helper — an authoritative, not inferred,
  source.)

- **Storage mode is configurable, and matters:** `AuthCredentialsStoreMode`
  (`codex-rs/config/src/types.rs`) has four variants — `File` (**the
  `#[default]`**), `Keyring`, `Auto` (keyring-if-available-else-file), and
  `Ephemeral` (memory-only, never touches disk). A **standard, unconfigured
  install uses `File`** — i.e. plaintext `auth.json`, same posture as Claude
  Code's Windows credential. This is not guaranteed forever or on every
  install: a user (or an organization's managed config) can set
  `preferred_auth_method`/store-mode to `Keyring` in `config.toml`, at which
  point `auth.json` may not exist or may not be the live source of truth at
  all. **Any `CredentialStore` implementation reading `auth.json` directly
  is therefore reading the default posture, not a structural guarantee** —
  the honest `Availability` outcome for "file absent because the user
  configured keyring storage" is indistinguishable, from a plain file read,
  from "never logged in," and both correctly map to `Unauthenticated` (a
  false negative here is not a lie — it just means "not available *this
  way*," which is exactly what `Unauthenticated` already means).
- The `id_token` JWT, once decoded (Codex does this itself; this adapter
  does not), carries `email`, `chatgpt_plan_type` (the ChatGPT subscription
  tier — free/plus/pro/business/enterprise/edu — Codex's rough analogue of
  Claude Code's `subscriptionType`), `chatgpt_user_id`, `chatgpt_account_id`.
  This adapter never decodes or exposes this JWT; it is documented here only
  because it establishes that a real account-identity signal *exists*
  in principle, for a future adapter revision.

## Usage signal

**CONFIRMED, and structurally different from Claude Code — there is no
local usage/quota cache to read.**

By source inspection of `codex-rs/codex-api/src/rate_limits.rs` and
`codex-rs/app-server/src/request_processors/account_processor.rs`:
rate-limit/usage data (`RateLimitSnapshot`) is parsed *only* from live HTTP
response headers returned by an authenticated call to OpenAI's backend
(`BackendClient::get_rate_limits_with_luna_reserve` /
`get_rate_limits_with_reset_credits`), or from response headers on ordinary
API calls. No code path in this repository writes a `RateLimitSnapshot`,
or anything usage-shaped, to a local file or to any of the local SQLite
databases (`state_5.sqlite`, `goals_1.sqlite`, `memories_1.sqlite`,
`queue_1.sqlite`, `thread_history_1.sqlite` — confirmed via `codex doctor`'s
own state-path inventory, and cross-checked with a grep across the source
tree for any persistence of `RateLimitSnapshot`, which found none).

**Consequence for this vertical slice, stated plainly per the task's own
instruction not to invent a signal that doesn't exist:** a Codex
`UsageSource` built without performing a live, authenticated network call
has no local number to report — not "hard to find," genuinely absent by
design. This adapter's `fetch()` reports `Availability::Unsupported` for
usage once authentication is confirmed present, rather than fabricating a
`Count`/`Fraction` from an unrelated local number (e.g. thread count) the
way that would misrepresent what was actually measured. This is precisely
the scenario `Unsupported` (see `core/src/domain/availability.rs`) exists
for: "this ... tool version cannot provide this signal at all" — here, not
because of a platform limitation, but a tool-inherent one, which the
existing `Availability` enum accommodates without needing a new variant.

## Activity signal

**PARTIALLY CONFIRMED.** A real, plausible activity signal *exists* in the
local `thread_history_1.sqlite` schema (`codex-rs/state/thread_history_migrations/0001_thread_history.sql`):
a `thread_turns` table with `status`, `started_at`, `completed_at`, and
`duration_ms` per turn — a turn with a `started_at` and no `completed_at`
is, in principle, a working/in-progress turn, which is exactly the shape
`ActivityState::Working` vs. `RecentlyIdle` needs.

**Not implemented in this vertical slice.** Reading it requires a
`SqliteReadOnlyOpener` capability that does not exist in `core/ports` yet,
and Claude Code's own adapter has no `ActivitySource` implementation either
(see `docs/design/claude-code-windows-local-state.md`) — adding one only for
Codex would make this slice's two tools asymmetric for a reason unrelated to
what's actually being tested (the tool/OS axis split), not because the
signal doesn't exist. This is deliberately left as documented future work,
not silently skipped: the schema confirms the signal is real and
addressable later behind the same `ActivitySource` port Claude Code will
also eventually need.

## Account discovery

**CONFIRMED.** Exactly one `auth.json` (hence one authenticated identity)
per `$CODEX_HOME`. Codex's own `--profile`/`-p` flag and `config.toml`
`[profiles]` map (`codex-rs/config/src/config_toml.rs`) are **named
configuration profiles** (model/provider/permission settings), not separate
accounts or credentials — confirmed by their type (`ConfigProfile`, no
credential fields) and the fact `auth.json` lives at the top of
`$CODEX_HOME` regardless of which profile is active. A second Codex account
on the same machine would require a second `$CODEX_HOME` directory (via the
`CODEX_HOME` env var), which Codex itself does not auto-discover — matching
exactly the same shape of limitation already documented for Claude Code
("discovery of more than one account/profile" — not yet built, same
reason).

## Stability

- The main state database alone has at least 54 numbered migrations
  (`codex-rs/state/migrations/0001_*.sql` through `0054_*.sql` at this
  version) — an actively evolving schema. Reading any SQLite table directly
  (not attempted by this adapter) would need to tolerate schema drift across
  Codex versions far more aggressively than Claude Code's single flat
  `stats-cache.json`.
- `AuthDotJson` itself looks comparatively stable (a flat struct with
  `#[serde(default, skip_serializing_if = "Option::is_none")]` on every
  optional field, which is specifically defensive against exactly this kind
  of drift), but it is an internal type with **no public schema guarantee**
  — a future Codex release could rename or restructure it without notice,
  same caveat as Claude Code's `.credentials.json`.
- `last_refresh` is an RFC3339 `DateTime<Utc>` (via `chrono`) — a plain
  string in the JSON, not a raw epoch number; a future parser needs an
  RFC3339-capable reader, not a bare integer parse.
- The configurable `AuthCredentialsStoreMode` (see above) is the single
  biggest migration/breakage risk specific to Codex: a `CredentialStore`
  that only ever reads `auth.json` will report `Unauthenticated` for a real,
  logged-in user who has configured `Keyring` or `Auto` mode and it fell
  through to the keyring. This is a known, named, accepted limitation of
  reading only the default posture — not a bug to silently paper over.

## Security

- The adapter (Phase C) never reads the actual `access_token`/`refresh_token`/
  `id_token` JWT values out of `auth.json` — exactly like the Claude Code
  adapter's relationship to `.credentials.json`, it only asks its injected
  `CredentialStore` whether the file is `Found`/`NotFound`/`AccessDenied`,
  and uses that outcome alone to decide `Availability`. The file's contents,
  if ever read, stay behind `SecretHandle`'s redacted `Debug` and are never
  logged.
- No write path exists anywhere in this adapter or the Linux `CredentialStore`
  implementation — both are read-only by construction (`std::fs::read_to_string`
  only).
- No network call is made by this adapter at all (a direct consequence of
  the "Usage signal" finding above) — there is nothing to redact from a
  request/response this build never sends.

## Confidence summary

| Claim | Confidence |
|---|---|
| `$CODEX_HOME` resolution (`CODEX_HOME` env var, else `$HOME/.codex`) | CONFIRMED (source) |
| `auth.json` shape (`AuthDotJson`) | CONFIRMED (source) |
| Default auth storage mode is a plaintext file, not OS keyring | CONFIRMED (source: `#[default]` on `AuthCredentialsStoreMode::File`) |
| No local usage/rate-limit cache exists; usage requires a live network call | CONFIRMED (source: `rate_limits.rs`, `account_processor.rs`, cross-repo grep for any local persistence found none) |
| `thread_turns` schema could support a future activity signal | PARTIALLY CONFIRMED (schema read directly; no adapter reads it in this slice, so its real-world reliability as an "is it working right now" signal is unverified) |
| Only one account per `$CODEX_HOME`; `--profile` is config, not identity | CONFIRMED (source: `ConfigProfile` has no credential fields) |
| Behavior of a real, logged-in account's `auth.json` on disk (exact byte-for-byte shape in practice, not just the struct definition) | UNKNOWN — no real login was performed, per the task's explicit instruction not to complete real authentication during this investigation |
