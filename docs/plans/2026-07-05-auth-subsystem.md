# Proserpina — Auth Subsystem Design (v0.3.0)

- **Date:** 2026-07-05
- **Branch:** `feature/v0.3.0-auth-design`
- **Target:** v0.3.0
- **Motivation:** Proserpina currently relies on env vars, credentials.toml, and
  pi discovery for auth. None of these let a user *obtain* a credential from the
  CLI — they all assume the key already exists somewhere. A standalone Proserpina
  user (no pi, no pre-existing keys) needs `proserpina auth login <provider>` to
  acquire, store, and refresh credentials interactively.

## 1. Scope

| In v0.3.0 | Deferred |
|---|---|
| `proserpina auth login <provider>` (interactive) | `proserpina auth revoke <provider>` |
| API-key providers (DeepSeek, Z.ai, DashScope, Google, Moonshot) | Multi-user / multi-tenant auth |
| OAuth provider (OpenAI Codex / ChatGPT) | Custom OAuth providers |
| Token storage in credentials.toml + keyring | Server-side auth (SAAS mode) |
| Token refresh lifecycle (OAuth) | MFA / hardware keys |
| ratatui TUI for the auth flow | |
| Pi discovery as convenience fallback (already built) | |

## 2. Auth Methods by Provider

Each provider has one of three auth methods. The CLI detects which one and runs
the appropriate flow:

| Provider | Method | Flow |
|---|---|---|
| DeepSeek | API key | Prompt for key, validate via GET /models, store |
| Z.ai | API key (+ login step) | Prompt for key OR run login flow, validate, store |
| DashScope (Alibaba) | API key | Prompt for key, validate, store |
| Google | API key | Prompt for key, validate, store |
| Moonshot (Kimi) | API key | Prompt for key, validate, store |
| OpenAI (Codex) | OAuth (PKCE) | Browser flow → token exchange → store access+refresh |

### API-key flow (most providers)

```
$ proserpina auth login deepseek
  Enter your DeepSeek API key (get one at https://platform.deepseek.com):
  > sk-********************************
  Validating... ✓
  Stored in keychain (proserpina:DEEPSEEK_API_KEY)
```

If keyring is available: stored in the OS keychain. If not: stored in
credentials.toml with a warning about plaintext.

### OAuth flow (OpenAI Codex)

Uses the same PKCE flow as OpenAI's official Codex CLI (and pi):
- CLIENT_ID: `app_EMoamEEZ73f0CkXaXp7hrann` (OpenAI's public CLI client_id)
- AUTHORIZE_URL: `https://auth.openai.com/oauth/authorize`
- TOKEN_URL: `https://auth.openai.com/oauth/token`
- PKCE S256 challenge
- Local callback server on port 1455
- Scope: `openid profile email offline_access`

```
$ proserpina auth login openai
  Opening browser for OpenAI authentication...
  (If the browser doesn't open, visit: https://auth.openai.com/oauth/authorize?...)
  
  Waiting for authentication... ✓
  Token stored (expires in 3600s; auto-refresh enabled)
```

Stored as `{type: "oauth", access: "...", refresh: "...", expires: <ms_epoch>}`.

### Z.ai login step

Z.ai with a coding plan may need the interactive login (not just an API key).
The CLI detects this: if a plain API key fails validation, it attempts the
login flow (which may involve a browser step or device-code flow depending on
Z.ai's current auth UI). The resulting key is stored like any API key.

## 3. Token Storage

### Credentials store

A single `~/.config/proserpina/credentials.toml` (or keychain if feature is on):

```toml
# API keys (plain text if no keyring; encrypted if keyring feature is on)
[deepseek]
api_key = "sk-..."

# OAuth tokens (with refresh lifecycle)
[openai]
type = "oauth"
access = "eyJ..."
refresh = "rt_1..."
expires = 1781648822320  # ms epoch
```

### Keyring integration (behind `keyring` feature)

API keys are stored as `proserpina:<PROVIDER>_API_KEY` in the OS keychain
(macOS Keychain, Windows Credential Manager). On Linux without a working
keyring backend, falls back to credentials.toml with a warning.

OAuth tokens are too large for keychain entries (access tokens can be 1.8KB+);
they go in credentials.toml regardless of keyring. Only the API keys benefit
from keychain storage.

### Token refresh lifecycle

On every `authed_configs_with()` call, OAuth tokens are checked:
1. If `expires > now` → use the stored access token.
2. If `expires <= now` → POST `grant_type=refresh_token` to the token endpoint.
   - Success → update the stored token, use the new access token.
   - Failure → mark the provider as not-authed (don't crash the run).

This runs transparently; the user sees `✓ openai (gpt-4o) (refreshed)` in
`auth check` output if a refresh happened.

## 4. ratatui TUI

The auth flow uses a ratatui TUI for a polished interactive experience:

```
┌─ Proserpina Auth ──────────────────────────────┐
│                                                │
│  Select a provider to authenticate:            │
│                                                │
│  > deepseek    API key                         │
│    openai      OAuth (ChatGPT)                 │
│    zai         API key / login                 │
│    dashscope   API key                         │
│    google      API key                         │
│    moonshot    API key                         │
│                                                │
│  ↑↓ navigate · Enter select · q quit           │
└────────────────────────────────────────────────┘
```

After selecting a provider:
- **API key**: a text input field with masked input + validation status.
- **OAuth**: a spinner + "browser opened" message + waiting-for-callback.
- **Login step**: provider-specific instructions + status.

`proserpina auth login` (no provider) opens the provider selector.
`proserpina auth login deepseek` skips straight to the API-key prompt.

## 5. CLI Subcommands

```
proserpina auth login [provider]     # interactive credential acquisition
proserpina auth check                # validate all stored keys/tokens
proserpina auth list                 # show which providers have credentials
proserpina auth status [provider]    # detailed status for one provider
proserpina auth logout [provider]    # remove stored credentials
```

## 6. Architecture

### TUI seam (trait-isolated for Knopper migration)

The auth logic depends only on a trait, never on the TUI library directly:

```rust
pub trait AuthUi {
    fn select_provider(&self, providers: &[ProviderInfo]) -> Option<usize>;
    fn prompt_api_key(&self, provider: &str, signup_url: &str) -> Result<String>;
    fn oauth_open_browser(&self, url: &str) -> Result<()>;
    fn oauth_wait_for_callback(&self) -> Result<()>;
    fn show_status(&self, message: &str);
    fn show_success(&self, message: &str);
    fn show_error(&self, message: &str);
}
```

Today: `RatatuiAuthUi` implements this trait. When Knopper matures, a
`KnopperAuthUi` replaces it with zero changes to the auth logic.

### New module: `src/auth/`

```
src/auth/
├── mod.rs          # AuthStore, ProviderAuth, AuthMethod, AuthUi trait
├── api_key.rs      # API-key flow (validate via GET /models)
├── oauth.rs        # OAuth PKCE flow (browser, token exchange, refresh)
├── store.rs        # credential storage (credentials.toml + keyring)
└── tui_ratatui.rs  # RatatuiAuthUi (the ONLY file that imports ratatui)
```

The auth logic (`api_key.rs`, `oauth.rs`, `store.rs`) imports only the
`AuthUi` trait from `mod.rs` — never ratatui directly. Swapping to Knopper is
adding `tui_knopper.rs` and changing one line in the CLI dispatch.

### AuthStore

```rust
pub struct AuthStore {
    credentials: Credentials,       // existing credentials.toml
    keyring_available: bool,
}

impl AuthStore {
    pub fn discover() -> Result<Self>;
    pub fn login(&mut self, provider: &str) -> Result<()>;
    pub fn logout(&mut self, provider: &str) -> Result<()>;
    pub fn check(&self) -> Vec<AuthStatus>;
    pub fn refresh_oauth(&mut self, provider: &str) -> Result<()>;
}
```

### Provider auth registry

```rust
enum AuthMethod {
    ApiKey { validate_url: String, signup_url: String },
    OAuth { client_id: String, authorize_url: String, token_url: String, scope: String },
    LoginStep { instructions: String },
}

struct ProviderAuth {
    name: &str,
    method: AuthMethod,
}
```

This replaces the hardcoded URL/model registry with an auth-focused registry.
The existing `Provider` in `roster.rs` stays for roster assignment; this is
the auth-side companion.

## 7. Dependencies

- `ratatui` — TUI for interactive flows (behind `cli` feature)
- `webbrowser` — open the browser for OAuth (behind `cli`)
- `base64` — PKCE challenge encoding (behind `cli`)
- `sha2` — PKCE S256 (behind `cli`)
- Existing: `reqwest`, `tokio`, `serde`, `serde_json`

## 8. Security

- API keys in the keychain are never written to disk in plaintext (when
  keyring feature is on).
- OAuth tokens (too large for keychain) are stored in credentials.toml —
  file permissions should be 0600. Proserpina sets this on first write.
- Tokens are never logged or printed. `auth list` shows `✓ set` not the value.
- The `expires` field uses ms-epoch (matching pi/OpenAI convention).

## 9. Pi Discovery Coexistence

Pi discovery (PR #24) stays as a convenience layer:
- Resolution priority: **Proserpina's own credentials > pi discovery > env
  vars > registry defaults**.
- If a user authenticates via `proserpina auth login deepseek`, that key
  takes precedence over any pi-discovered key.
- `proserpina auth check` shows BOTH sources (e.g. `deepseek ✓
  (proserpina store)` vs `deepseek ✓ (pi discovery)`).

## 10. Out of Scope for v0.3.0

- Schubert integration (multi-user authorization — different problem).
- Batch auth (auth all providers at once).
- SAAS / server-side auth.
- Device-code flow (for headless machines without a browser).
- Custom OAuth providers (user-defined OAuth configs).
