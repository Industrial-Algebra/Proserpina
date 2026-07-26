// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! The credentials config: a standalone file mapping provider names to API
//! keys and optional model/base_url overrides.
//!
//! Proserpina reads `~/.config/proserpina/credentials.toml` (location overridable via
//! `PROSERPINA_CONFIG` or `--config`) so it can reach providers whose keys are not
//! in the environment — pi mediates several providers via OAuth/extensions and
//! does not expose plain keys to a separate process. The same file also lets
//! you override the registry's model defaults (e.g. pin a specific Z.ai or
//! OpenAI model), and define custom providers (any OpenAI-compatible
//! endpoint: Ollama, LM Studio, OpenRouter, a proxy).
//!
//! Resolution (env > config > registry-default) and the pure
//! `resolve_providers` core land in a follow-up step; this module ships the
//! config data model, parsing, and file discovery.

use std::collections::HashMap;
use std::path::Path;

use crate::backend::http::HttpConfig;
use crate::backend::roster::Provider;
use crate::error::ProserpinaError;

/// A per-provider override block parsed from the config file.
///
/// All fields optional at the config layer; required-ness is enforced during
/// resolution (a custom provider not in the registry must supply all three).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ProviderOverride {
    /// The API key. Present => the provider is authed (subject to env override).
    pub api_key: Option<String>,
    /// Override the registry's default model.
    pub model: Option<String>,
    /// Override the registry's default base URL.
    pub base_url: Option<String>,
    /// OAuth access token (for OAuth-type providers like OpenAI Codex).
    /// When present, used as the bearer token instead of `api_key`.
    pub oauth_access: Option<String>,
    /// OAuth refresh token for refreshing expired access tokens.
    pub oauth_refresh: Option<String>,
    /// OAuth access token expiry, in milliseconds since Unix epoch.
    pub oauth_expires: Option<u64>,
}

/// A user-defined panel from the config file: a named list of personas.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct PanelConfig {
    /// The personas in this panel.
    pub personas: Vec<PersonaSpec>,
}

/// The `[retry]` section of the config file, re-exported from
/// `proserpina-agent` (where [`RetryPolicy`](crate::backend::http::RetryPolicy)
/// lives). All fields optional; missing fields fall back to
/// [`crate::backend::http::RetryPolicy::DEFAULT`] at resolution time.
pub use proserpina_agent::http::RetryConfig;

/// One persona in a config-defined panel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PersonaSpec {
    pub name: String,
    pub framing: Option<String>,
    pub focus: Option<String>,
}

impl PersonaSpec {
    /// Converts this spec into a [`crate::persona::Persona`].
    pub fn to_persona(&self) -> crate::persona::Persona {
        let mut p = crate::persona::Persona::new(self.name.clone());
        if let Some(f) = &self.framing {
            p = p.with_framing(f.clone());
        }
        if let Some(f) = &self.focus {
            p = p.with_focus(f.clone());
        }
        p
    }
}

/// The parsed credentials config: provider name → override block, plus any
/// user-defined panels under `[panels.NAME]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Credentials {
    providers: HashMap<String, ProviderOverride>,
    panels: HashMap<String, PanelConfig>,
    retry: RetryConfig,
    /// Model names to exclude from the provider pool (e.g. `"qwen3.7-max"`
    /// when billing is disabled). Matched against each config's model.
    exclude: Vec<String>,
}

impl Credentials {
    /// Parses credentials from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`ProserpinaError::MalformedCredentials`] if the TOML is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::backend::credentials::Credentials;
    /// let creds = Credentials::from_toml(r#"[deepseek]
    /// api_key = "sk-x""#).expect("valid toml");
    /// assert_eq!(creds.override_for("deepseek").unwrap().api_key.as_deref(), Some("sk-x"));
    /// ```
    pub fn from_toml(toml: &str) -> Result<Self, ProserpinaError> {
        if toml.trim().is_empty() {
            return Ok(Self::default());
        }
        // Parse with explicit `panels`, `retry`, and `exclude` tables; everything
        // else is a provider section (flattened). This keeps `[panels.NAME]`
        // separate from `[provider-name]`.
        #[derive(serde::Deserialize)]
        struct Raw {
            #[serde(default)]
            panels: HashMap<String, PanelConfig>,
            #[serde(default)]
            retry: RetryConfig,
            #[serde(default)]
            exclude: Vec<String>,
            #[serde(flatten)]
            providers: HashMap<String, ProviderOverride>,
        }
        let parsed: Raw = ::toml::from_str(toml)
            .map_err(|e| ProserpinaError::malformed_credentials("<str>", e))?;
        Ok(Self {
            providers: parsed.providers,
            panels: parsed.panels,
            retry: parsed.retry,
            exclude: parsed.exclude,
        })
    }

    /// Reads and parses credentials from a file.
    ///
    /// # Errors
    ///
    /// Returns [`ProserpinaError::MalformedCredentials`] if the file cannot be
    /// read or parsed.
    pub fn from_path(path: &Path) -> Result<Self, ProserpinaError> {
        let display = path.display().to_string();
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ProserpinaError::malformed_credentials(&display, e))?;
        Self::from_toml(&contents).map_err(|e| ProserpinaError::malformed_credentials(&display, e))
    }

    /// Discovers the default config file and loads it.
    ///
    /// Searches in order: `$PROSERPINA_CONFIG`, then
    /// `$XDG_CONFIG_HOME/proserpina/credentials.toml`, then
    /// `~/.config/proserpina/credentials.toml`. A **missing** file is not an
    /// error — returns empty credentials so the run degrades gracefully to
    /// env-var-only auth.
    ///
    /// # Errors
    ///
    /// Returns [`ProserpinaError::MalformedCredentials`] only if a discovered
    /// file exists but cannot be read or parsed.
    pub fn discover() -> Result<Self, ProserpinaError> {
        for path in Self::candidate_paths() {
            if path.exists() {
                return Self::from_path(&path);
            }
        }
        Ok(Self::default())
    }

    /// Returns the ordered list of config-file candidate paths.
    fn candidate_paths() -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();
        if let Ok(p) = std::env::var("PROSERPINA_CONFIG") {
            paths.push(std::path::PathBuf::from(p));
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            paths.push(std::path::PathBuf::from(xdg).join("proserpina/credentials.toml"));
        }
        if let Ok(home) = std::env::var("HOME") {
            paths.push(std::path::PathBuf::from(home).join(".config/proserpina/credentials.toml"));
        }
        paths
    }

    /// The override block for `name`, if present.
    pub fn override_for(&self, name: &str) -> Option<&ProviderOverride> {
        self.providers.get(name)
    }

    /// Sets or replaces an override for a provider (write access for AuthStore).
    pub fn set_override(&mut self, name: &str, override_: ProviderOverride) {
        self.providers.insert(name.to_owned(), override_);
    }

    /// Merges a model override into an existing provider entry (preserving
    /// other fields like api_key or oauth tokens). Creates the entry if absent.
    pub fn merge_model(&mut self, name: &str, model: &str) {
        self.providers.entry(name.to_owned()).or_default().model = Some(model.to_owned());
    }

    /// Removes an override for a provider.
    pub fn remove_override(&mut self, name: &str) {
        self.providers.remove(name);
    }

    /// Serializes back to a TOML string (for AuthStore::save).
    pub fn to_toml_string(&self) -> String {
        // Use serde to serialize the providers map + panels + retry.
        // For now, a simple manual serialize since we need control over format.
        let mut out = String::new();
        for (name, ov) in &self.providers {
            out.push_str(&format!("[{name}]\n"));
            if let Some(k) = &ov.api_key {
                out.push_str(&format!("api_key = \"{k}\"\n"));
            }
            if let Some(m) = &ov.model {
                out.push_str(&format!("model = \"{m}\"\n"));
            }
            if let Some(u) = &ov.base_url {
                out.push_str(&format!("base_url = \"{u}\"\n"));
            }
            if let Some(t) = &ov.oauth_access {
                out.push_str(&format!("oauth_access = \"{t}\"\n"));
            }
            if let Some(r) = &ov.oauth_refresh {
                out.push_str(&format!("oauth_refresh = \"{r}\"\n"));
            }
            if let Some(e) = ov.oauth_expires {
                out.push_str(&format!("oauth_expires = {e}\n"));
            }
            out.push('\n');
        }
        if !self.panels.is_empty() {
            // Panels are complex; skip for now (they round-trip via from_toml).
        }
        if out.is_empty() {
            out = "# Proserpina credentials\n".to_owned();
        }
        out
    }

    /// Whether the config is empty (no provider sections).
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// An iterator over `(name, override)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &ProviderOverride)> {
        self.providers.iter()
    }

    /// Loads credentials from `path` if given, else discovers the default.
    ///
    /// Convenience for CLI entry points that take an optional `--config`.
    ///
    /// # Errors
    ///
    /// See [`Credentials::from_path`] and [`Credentials::discover`].
    pub fn discover_or(path: Option<&std::path::Path>) -> Result<Self, ProserpinaError> {
        match path {
            Some(p) => Self::from_path(p),
            None => Self::discover(),
        }
    }

    /// The user-defined panels in this config (name → panel).
    pub fn panels(&self) -> &HashMap<String, PanelConfig> {
        &self.panels
    }

    /// The `[retry]` section, if present (all-`None` if absent).
    pub fn retry(&self) -> &RetryConfig {
        &self.retry
    }

    /// The model names to exclude from the provider pool.
    pub fn exclude(&self) -> &[String] {
        &self.exclude
    }
}

/// Derives the conventional env-var name for a custom provider.
///
/// A custom provider `my-local-llm` falls back to `MY_LOCAL_LLM_API_KEY`.
fn env_var_name_for(provider_name: &str) -> String {
    format!("{}_API_KEY", provider_name.replace('-', "_").to_uppercase())
}

/// Resolves the authed, effective HTTP configs given a registry, the
/// credentials config, and an explicit snapshot of relevant environment keys.
///
/// This is the pure core of credential resolution. Precedence for each field:
/// - `api_key`: env var → config `api_key` → none (provider is *authed* iff a
///   key resolved)
/// - `model` / `base_url`: config override → registry default
///
/// Custom providers (config sections not matching a registry entry) must
/// supply all of `api_key`, `model`, and `base_url`. Returns `Vec<HttpConfig>`
/// directly — what [`crate::backend::roster::random_roster`] consumes.
///
/// Passing `env_keys` explicitly (rather than reading `std::env` inside) makes
/// resolution fully deterministic and unit-testable.
///
/// # Errors
///
/// Returns [`ProserpinaError::IncompleteCustomProvider`] if a custom provider is
/// missing a required field.
pub fn resolve_configs(
    registry: &[Provider],
    credentials: &Credentials,
    env_keys: &HashMap<String, String>,
) -> Result<Vec<HttpConfig>, ProserpinaError> {
    resolve_configs_with_keyring(registry, credentials, env_keys, &HashMap::new())
}

/// Resolves the authed, effective HTTP configs with a keyring tier at the
/// **highest** precedence: keyring > env > config-file > registry-default.
///
/// `keyring_keys` is a snapshot of the OS keychain, keyed the same way as
/// `env_keys` (by the provider's `key_env_var`, e.g. `DEEPSEEK_API_KEY`).
/// Passing it explicitly (rather than reading the keychain inside) keeps this
/// function pure and unit-testable; see [`read_keyring`] for the real-keychain
/// read (behind the `keyring` feature).
///
/// # Errors
///
/// Returns [`ProserpinaError::IncompleteCustomProvider`] if a custom provider is
/// missing a required field.
pub fn resolve_configs_with_keyring(
    registry: &[Provider],
    credentials: &Credentials,
    env_keys: &HashMap<String, String>,
    keyring_keys: &HashMap<String, String>,
) -> Result<Vec<HttpConfig>, ProserpinaError> {
    let registry_names: std::collections::HashSet<&str> =
        registry.iter().map(|p| p.name()).collect();
    let mut out = Vec::new();

    // 1. Registry providers, possibly overridden by config.
    for reg in registry {
        let cfg = credentials.override_for(reg.name());
        let key_var = reg.key_env_var();
        // OAuth access token (if stored) takes priority; then keyring > env > config api_key.
        let api_key = cfg
            .and_then(|c| c.oauth_access.clone())
            .or_else(|| keyring_keys.get(key_var).cloned())
            .or_else(|| env_keys.get(key_var).cloned())
            .or_else(|| cfg.and_then(|c| c.api_key.clone()));
        let Some(api_key) = api_key else {
            continue; // not authed
        };
        let model = cfg
            .and_then(|c| c.model.clone())
            .unwrap_or_else(|| reg.model().to_owned());
        let base_url = cfg
            .and_then(|c| c.base_url.clone())
            .unwrap_or_else(|| reg.base_url().to_owned());
        out.push(HttpConfig {
            base_url,
            model,
            api_key,
        });
    }

    // 2. Custom providers (config sections not matching any registry entry).
    for (name, cfg) in credentials.iter() {
        if registry_names.contains(name.as_str()) {
            continue;
        }
        let env_var = env_var_name_for(name);
        let api_key = keyring_keys
            .get(&env_var)
            .cloned()
            .or_else(|| env_keys.get(&env_var).cloned())
            .or(cfg.api_key.clone());
        let model = cfg.model.clone();
        let base_url = cfg.base_url.clone();
        let mut missing: Vec<&'static str> = Vec::new();
        if api_key.is_none() {
            missing.push("api_key");
        }
        if model.is_none() {
            missing.push("model");
        }
        if base_url.is_none() {
            missing.push("base_url");
        }
        if !missing.is_empty() {
            return Err(ProserpinaError::incomplete_custom_provider(name, missing));
        }
        out.push(HttpConfig {
            base_url: base_url.unwrap(),
            model: model.unwrap(),
            api_key: api_key.unwrap(),
        });
    }

    Ok(out)
}

/// Same as [`authed_configs`] but with an explicit config-file path override.
///
/// If `config_path` is `Some`, that path is used (and must exist). If `None`,
/// normal discovery applies.
///
/// # Errors
///
/// See [`authed_configs`].
pub fn authed_configs_with(
    config_path: Option<&std::path::Path>,
) -> Result<Vec<HttpConfig>, ProserpinaError> {
    authed_configs_with_excludes(config_path, &[])
}

/// Like [`authed_configs_with`] but also excludes the given model names (from
/// `--exclude`). Both config-level and CLI-level excludes are applied.
pub fn authed_configs_with_excludes(
    config_path: Option<&std::path::Path>,
    cli_excludes: &[String],
) -> Result<Vec<HttpConfig>, ProserpinaError> {
    let credentials = match config_path {
        Some(path) => Credentials::from_path(path)?,
        None => Credentials::discover()?,
    };
    let mut env_keys: HashMap<String, String> = HashMap::new();
    for reg in Provider::registry() {
        if let Ok(v) = std::env::var(reg.key_env_var()) {
            env_keys.insert(reg.key_env_var().to_owned(), v);
        }
    }
    for name in credentials.iter().map(|(n, _)| n.as_str()) {
        let var = env_var_name_for(name);
        if let Ok(v) = std::env::var(&var) {
            env_keys.insert(var, v);
        }
    }
    // The keyring tier (highest precedence) when the feature is on; empty
    // snapshot otherwise (compile-time branch, no runtime cost when off).
    #[cfg(feature = "keyring")]
    let keyring_keys = read_keyring_snapshot(&credentials);
    #[cfg(not(feature = "keyring"))]
    let keyring_keys: HashMap<String, String> = HashMap::new();
    let mut configs =
        resolve_configs_with_keyring(Provider::registry(), &credentials, &env_keys, &keyring_keys)?;

    // Merge in pi's discovered configs (correct per-user URLs/models that the
    // hardcoded registry may not know). Dedupe by provider host: when a pi
    // config has the same host as a registry config, the pi config REPLACES
    // the registry one (pi has the user's actual current model + URL).
    let pi_configs = discover_pi_configs();
    let pi_hosts: std::collections::HashSet<String> = pi_configs
        .iter()
        .map(|c| extract_host(&c.base_url))
        .collect();
    // Remove registry configs whose host is also served by a pi config.
    configs.retain(|c| !pi_hosts.contains(&extract_host(&c.base_url)));
    // Add all pi configs.
    configs.extend(pi_configs);

    // Apply excludes: config-level (`exclude = [...]`) + CLI-level (`--exclude`).
    let config_excludes = credentials.exclude();
    configs.retain(|c| !config_excludes.contains(&c.model) && !cli_excludes.contains(&c.model));

    Ok(configs)
}

/// Discovers provider configs from pi's `~/.pi/agent/models.json`, if pi is
/// installed. Returns authed `HttpConfig`s for each provider whose referenced
/// env var is set.
///
/// pi maintains correct per-user provider configs (baseUrl, model, key env-var
/// ref). Proserpina reads these to avoid maintaining a competing hardcoded
/// registry that drifts. If pi isn't installed, returns empty.
///
/// Also reads pi's `auth.json` — for providers that need a login step (Z.ai,
/// DashScope, Google), pi stores the actual working API key there. Those keys
/// are injected into the resolution so they work even when pi isn't running or
/// hasn't exported the env var.
pub fn discover_pi_configs() -> Vec<HttpConfig> {
    // Find pi's config: $PI_HOME/agent/models.json or ~/.pi/agent/models.json
    let pi_home = std::env::var("PI_HOME")
        .ok()
        .or_else(|| std::env::var("HOME").ok().map(|h| format!("{h}/.pi")));
    let Some(pi_home) = pi_home else {
        return Vec::new();
    };
    let models_path = std::path::Path::new(&pi_home).join("agent/models.json");
    let Ok(contents) = std::fs::read_to_string(&models_path) else {
        return Vec::new(); // pi not installed or no models.json
    };
    let Ok(parsed): Result<serde_json::Value, _> = serde_json::from_str(&contents) else {
        return Vec::new(); // malformed — skip silently
    };
    let Some(providers) = parsed.get("providers").and_then(|p| p.as_object()) else {
        return Vec::new();
    };

    // Read pi's auth.json for login-step keys (Z.ai, DashScope, Google, etc.).
    // These are the actual working keys that pi's login obtains.
    let auth_keys = read_pi_auth_keys(&pi_home);

    let mut configs = Vec::new();
    for (_name, provider) in providers {
        let base_url = provider.get("baseUrl").and_then(|v| v.as_str());
        let api_key_ref = provider.get("apiKey").and_then(|v| v.as_str());
        let models = provider.get("models").and_then(|m| m.as_array());
        let api = provider.get("api").and_then(|v| v.as_str());

        // Only OpenAI-compatible providers.
        if api != Some("openai-completions") {
            continue;
        }

        let (Some(base_url), Some(api_key_ref), Some(models)) = (base_url, api_key_ref, models)
        else {
            continue;
        };

        // Resolve the key: pi uses "$ENV_VAR" refs.
        // Priority: auth.json key > env var.
        let key_var = api_key_ref.strip_prefix('$').unwrap_or(api_key_ref);
        let api_key = auth_keys
            .get(key_var)
            .cloned()
            .or_else(|| std::env::var(key_var).ok());
        let Some(api_key) = api_key else {
            continue; // not authed
        };

        // Use the first model's id.
        let model = models
            .first()
            .and_then(|m| m.get("id").and_then(|id| id.as_str()))
            .unwrap_or("default");

        configs.push(HttpConfig {
            base_url: base_url.to_owned(),
            model: model.to_owned(),
            api_key,
        });
    }
    configs
}

/// Reads pi's `auth.json` and extracts the API keys from `type="api_key"`
/// entries. Returns a map of env-var-name -> key, so the key resolution can
/// look them up by the `$ENV_VAR` reference in models.json.
///
/// The mapping from auth.json entry name to env var name:
/// - `dashscope` -> `DASHSCOPE_API_KEY`
/// - `zai-coding-cn` -> `ZAI_API_KEY` (strip `-coding-cn`, or known mapping)
/// - `google` -> `GOOGLE_API_KEY`
///
/// OAuth entries (`type="oauth"`) are skipped — their access tokens are
/// time-limited and need pi's runtime to refresh.
fn read_pi_auth_keys(pi_home: &str) -> HashMap<String, String> {
    let auth_path = std::path::Path::new(pi_home).join("agent/auth.json");
    let Ok(contents) = std::fs::read_to_string(&auth_path) else {
        return HashMap::new();
    };
    let Ok(parsed): Result<serde_json::Value, _> = serde_json::from_str(&contents) else {
        return HashMap::new();
    };
    let Some(entries) = parsed.as_object() else {
        return HashMap::new();
    };

    let mut out = HashMap::new();
    for (name, creds) in entries {
        // Only type="api_key" entries.
        let cred_type = creds.get("type").and_then(|t| t.as_str());
        if cred_type != Some("api_key") {
            continue;
        }
        let Some(key) = creds.get("key").and_then(|k| k.as_str()) else {
            continue;
        };

        // Map auth.json entry name to env var name.
        let env_var = auth_entry_to_env_var(name);
        out.insert(env_var, key.to_owned());
    }
    out
}

/// Extracts the hostname from a base_url for provider-dedup purposes.
/// `https://api.deepseek.com/v1` → `api.deepseek.com`
/// `https://api.deepseek.com` → `api.deepseek.com`
fn extract_host(url: &str) -> String {
    let no_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let no_port = no_scheme.split(':').next().unwrap_or(no_scheme);
    let no_path = no_port.split('/').next().unwrap_or(no_port);
    no_path.to_owned()
}

/// Maps a pi auth.json entry name to the corresponding env var name.
/// `dashscope` -> `DASHSCOPE_API_KEY`, `zai-coding-cn` -> `ZAI_API_KEY`, etc.
fn auth_entry_to_env_var(name: &str) -> String {
    // Known mappings for pi-specific entry names.
    let base = match name {
        "zai-coding-cn" => "zai",
        "openai-codex" => "openai",
        other => other,
    };
    format!("{}_API_KEY", base.replace('-', "_").to_uppercase())
}

/// The thin CLI-facing wrapper over [`resolve_configs`]: discovers the
/// credentials config, snapshots the real environment, and resolves against
/// the built-in provider registry.
///
/// Returns an empty `Vec` (not an error) when nothing is authed — surfacing
/// [`ProserpinaError::NoAuthedProviders`] is the caller's job, since the right
/// response depends on context (the CLI errors; a library caller may proceed).
///
/// # Errors
///
/// Returns [`ProserpinaError::MalformedCredentials`] if a discovered config file
/// is unreadable or unparseable, or [`ProserpinaError::IncompleteCustomProvider`]
/// if a custom provider is missing required fields.
pub fn authed_configs() -> Result<Vec<HttpConfig>, ProserpinaError> {
    authed_configs_with(None)
}

// ---- OS keychain tier (behind the `keyring` feature) ----

/// The keychain service name Proserpina stores keys under. An entry for a
/// provider is looked up as `proserpina:<its key env var>` (e.g. `proserpina:DEEPSEEK_API_KEY`).
#[cfg(feature = "keyring")]
pub const KEYRING_SERVICE: &str = "proserpina";

/// Reads one provider's key from the OS keychain, keyed by its env-var name.
///
/// Returns `Ok(Some(key))` if an entry exists under `proserpina:<key_env_var>`,
/// `Ok(None)` if no entry exists, or `Err` if the keychain itself is
/// inaccessible (no Secret Service on a headless Linux box, etc.). A missing
/// entry is the normal case (most providers won't have one); a keychain error
/// is logged-but-skipped at the caller so a broken keychain doesn't sink the
/// whole run.
///
/// # Errors
///
/// Returns [`ProserpinaError::KeyringAccess`] if the keychain backend errors.
#[cfg(feature = "keyring")]
pub fn read_keyring(key_env_var: &str) -> Result<Option<String>, ProserpinaError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, key_env_var)
        .map_err(|e| ProserpinaError::keyring_access(key_env_var, e))?;
    match entry.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(ProserpinaError::keyring_access(key_env_var, e)),
    }
}

/// Builds the keyring snapshot for [`authed_configs_with`]: reads the
/// keychain for every registry provider's env var + every custom provider's
/// derived env var. Failures are skipped (a broken keychain for one entry
/// doesn't sink the run).
#[cfg(feature = "keyring")]
fn read_keyring_snapshot(credentials: &Credentials) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for reg in Provider::registry() {
        let var = reg.key_env_var();
        if let Ok(Some(v)) = read_keyring(var) {
            out.insert(var.to_owned(), v);
        }
    }
    for name in credentials.iter().map(|(n, _)| n.as_str()) {
        let var = env_var_name_for(name);
        if let Ok(Some(v)) = read_keyring(&var) {
            out.insert(var, v);
        }
    }
    out
}
