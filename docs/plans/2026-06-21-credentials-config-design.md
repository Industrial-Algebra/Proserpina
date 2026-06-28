# Proserpina — Credentials Config Design

- **Date:** 2026-06-21
- **Status:** Approved (design phase; implementation via TDD)
- **Branch:** `feature/credentials-config`
- **Depends on:** `backend-http` feature (PR #6), multi-provider roster (PR #7)

## 1. Purpose

The roster (PR #7) filters providers by API keys present as environment
variables. In practice only DeepSeek (and Z.ai) have plain env-var keys —
pi mediates the others (OpenAI, Google) via OAuth and some (Moonshot) via
specialized extensions, none of which expose plain keys to a separate process
like Proserpina. So the roster's diversity value is undermined: without reachable
keys, it degrades to "DeepSeek (+ Z.ai when it has balance)."

This design adds a **standalone credentials config file** so Proserpina can reach
all six providers (and any custom OpenAI-compatible endpoint). As a bonus it
also solves the **model-drift** problem flagged in PR #7: the same file lets
you override the registry's `glm-4-plus` with the current `glm-5.2`, pick a
preferred OpenAI model, etc., without code changes.

## 2. Key Design Decisions

1. **Standalone config file, not pi-integration.** Proserpina reads its own
   `~/.config/proserpina/credentials.toml`; it does not couple to pi's credential
   storage. You obtain each key once from the provider dashboard. Standard
   multi-CLI pattern; keeps Proserpina decoupled and publishable.
2. **Per-provider sections with optional model/base_url overrides.** One file
   for auth *and* personalization. Solves model drift in the same feature.
3. **Custom-provider support.** A config section whose name doesn't match a
   registry provider is treated as a custom provider (must supply `base_url`
   + `model` + `api_key`). This lets Proserpina reach any OpenAI-compatible
   endpoint — Ollama, LM Studio, OpenRouter, a proxy — not just the six
   presets.
4. **Resolution precedence: env > config > registry-default.** Env vars are
   ephemeral/override/CI-friendly; the config file is the persistent default;
   registry defaults fill the rest. So an env var temporarily overrides the
   config, and the config overrides the registry's model/base_url defaults.
5. **Pure resolution core for testability.** `resolve_providers` takes an
   explicit env-key snapshot (a `HashMap`) plus the config plus the registry,
   so it is fully unit-testable without touching the real environment — same
   pattern that kept `random_roster` clean.

## 3. Config File

**Location** (first found wins):
1. `--config <path>` CLI flag (explicit override)
2. `PROSERPINA_CONFIG` environment variable
3. `$XDG_CONFIG_HOME/proserpina/credentials.toml`
4. `~/.config/proserpina/credentials.toml`

A missing config file is **not an error** — Proserpina proceeds with whatever env
keys are available (degrading gracefully to the PR #7 behavior). A malformed
file *is* an error.

**Format** (TOML — IA convention, matches Schubert's `policy` feature):

```toml
# Registry providers: api_key makes them authed; model/base_url optional.
[deepseek]
api_key = "sk-..."

[zai]
api_key = "..."
model = "glm-5.2"            # override the drifted registry default

[openai]
api_key = "sk-..."
model = "gpt-4o-mini"

# Custom provider (name not in the registry): all three fields required.
[my-local-llm]
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama"
```

## 4. Resolution

For each provider name in (registry ∪ config sections):

| field | source (highest precedence first) |
|---|---|
| `api_key` | env var (`<NAME>_API_KEY` per registry, else none) → config `api_key` → none |
| `model` | config `model` → registry default → **error** for custom providers missing it |
| `base_url` | config `base_url` → registry default → **error** for custom providers missing it |

A provider is **authed** iff `api_key` resolved. Authed providers become
`HttpConfig`s fed to the roster. Resolution is a pure function of
`(registry, credentials, env_snapshot)`.

### Env-var name derivation

Registry providers declare their env var (`DEEPSEEK_API_KEY`, etc.). Custom
providers don't have a registry entry, so their env-var fallback (if desired)
is `<UPPERCASE_NAME>_API_KEY` — e.g. `MY_LOCAL_LLM_API_KEY`. (Minor; custom
providers will usually get their key from the config section directly.)

## 5. Crate Shape

- New module `src/backend/credentials.rs`, gated `#[cfg(feature = "backend-http")]`.
- `toml` dependency added behind `backend-http` (serde already implied by it).
- `Provider` gains `env_key_for(name)` helper for custom-provider env-var
  derivation (or this lives in credentials.rs — TBD during implementation).
- `random_roster` (pure) is **untouched**. Only the provider-source layer
  beneath it changes: the CLI's `run_critique` resolves providers via the new
  `authed_providers()` instead of `Provider::registry()` + `config_from_env`.

## 6. API Surface

```rust
/// A per-provider override block from the config file.
pub struct ProviderOverride {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

/// The parsed credentials config.
pub struct Credentials { providers: HashMap<String, ProviderOverride> }

impl Credentials {
    pub fn from_str(toml: &str) -> Result<Self, ProserpinaError>;
    pub fn from_path(path: &Path) -> Result<Self, ProserpinaError>;
    pub fn discover() -> Result<Self, ProserpinaError>;  // finds the default path; missing = empty
}

/// Pure: resolve the authed, effective providers given registry + config + env.
pub fn resolve_providers(
    registry: &[Provider],
    credentials: &Credentials,
    env_keys: &HashMap<String, String>,  // env var name -> value
) -> Result<Vec<Provider>, ProserpinaError>;

/// Convenience for the CLI: discover config + read real env + resolve.
pub fn authed_providers() -> Result<Vec<Provider>, ProserpinaError>;
```

New error variants:
- `ProserpinaError::MalformedCredentials { path, detail }`
- `ProserpinaError::IncompleteCustomProvider { name, missing }` (custom section
  missing required `base_url`/`model`/`api_key`)

## 7. CLI Integration

- `proserpina critique` gains `--config <path>` (optional; overrides discovery).
- The roster path (`run_critique`) calls `authed_providers()` instead of
  filtering `Provider::registry()` by `config_from_env`.

## 8. Implementation Sequencing (TDD)

Each step RED → GREEN → REFACTOR, every public item documented.

1. **`ProviderOverride` + `Credentials::from_str`** — parse TOML; pure.
2. **`Credentials::from_path` / `discover`** — file discovery + read; tested
   via `tempfile` (added as a dev-dependency). Missing default file → empty.
3. **`resolve_providers` pure core** — env > config > registry precedence;
   model/base_url override merging; authed filtering; custom-provider
   validation (missing required field → error). All via an explicit env
   snapshot, no real env.
4. **`authed_providers` convenience + roster wiring** — the CLI's `run_critique`
   uses it; existing roster tests stay green.
5. **CLI `--config` flag** — passes the path through; end-to-end.

## 9. Open Questions / Future Work

- **`--api-key` / `--provider` CLI flags** for ad-hoc single-provider runs
  without editing the config (deferred; `--config` + env cover the need).
- **`[default]` section** for global settings (default model, default seed) —
  YAGNI for now.
- **Keychain integration** (`keyring` crate) as an additional credential
  source tier — more secure than a plaintext file, but adds a platform dep.
- **Credentials file permissions check / advisory** — warn if world-readable.
