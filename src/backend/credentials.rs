// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The credentials config: a standalone file mapping provider names to API
//! keys and optional model/base_url overrides.
//!
//! Praxis reads `~/.config/praxis/credentials.toml` (location overridable via
//! `PRAXIS_CONFIG` or `--config`) so it can reach providers whose keys are not
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
use crate::error::PraxisError;

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
}

/// A user-defined panel from the config file: a named list of personas.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct PanelConfig {
    /// The personas in this panel.
    pub personas: Vec<PersonaSpec>,
}

/// The `[retry]` section of the config file. All fields optional; missing
/// fields fall back to [`crate::backend::http::RetryPolicy::DEFAULT`] at
/// resolution time.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct RetryConfig {
    /// Total tries including the first.
    pub max_attempts: Option<u32>,
    /// Per-attempt socket+read timeout, in seconds.
    pub timeout_secs: Option<u64>,
    /// Backoff before the second attempt, in milliseconds.
    pub initial_backoff_ms: Option<u64>,
    /// Exponential growth factor between backoffs.
    pub backoff_factor: Option<f64>,
    /// Cap on any single backoff, in milliseconds.
    pub max_backoff_ms: Option<u64>,
}

/// One persona in a config-defined panel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PersonaSpec {
    name: String,
    framing: Option<String>,
    focus: Option<String>,
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
}

impl Credentials {
    /// Parses credentials from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`PraxisError::MalformedCredentials`] if the TOML is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::backend::credentials::Credentials;
    /// let creds = Credentials::from_toml(r#"[deepseek]
    /// api_key = "sk-x""#).expect("valid toml");
    /// assert_eq!(creds.override_for("deepseek").unwrap().api_key.as_deref(), Some("sk-x"));
    /// ```
    pub fn from_toml(toml: &str) -> Result<Self, PraxisError> {
        if toml.trim().is_empty() {
            return Ok(Self::default());
        }
        // Parse with an explicit `panels` table; everything else is a provider
        // section (flattened). This keeps `[panels.NAME]` separate from
        // `[provider-name]`.
        #[derive(serde::Deserialize)]
        struct Raw {
            #[serde(default)]
            panels: HashMap<String, PanelConfig>,
            #[serde(default)]
            retry: RetryConfig,
            #[serde(flatten)]
            providers: HashMap<String, ProviderOverride>,
        }
        let parsed: Raw =
            ::toml::from_str(toml).map_err(|e| PraxisError::malformed_credentials("<str>", e))?;
        Ok(Self {
            providers: parsed.providers,
            panels: parsed.panels,
            retry: parsed.retry,
        })
    }

    /// Reads and parses credentials from a file.
    ///
    /// # Errors
    ///
    /// Returns [`PraxisError::MalformedCredentials`] if the file cannot be
    /// read or parsed.
    pub fn from_path(path: &Path) -> Result<Self, PraxisError> {
        let display = path.display().to_string();
        let contents = std::fs::read_to_string(path)
            .map_err(|e| PraxisError::malformed_credentials(&display, e))?;
        Self::from_toml(&contents).map_err(|e| PraxisError::malformed_credentials(&display, e))
    }

    /// Discovers the default config file and loads it.
    ///
    /// Searches in order: `$PRAXIS_CONFIG`, then
    /// `$XDG_CONFIG_HOME/praxis/credentials.toml`, then
    /// `~/.config/praxis/credentials.toml`. A **missing** file is not an
    /// error — returns empty credentials so the run degrades gracefully to
    /// env-var-only auth.
    ///
    /// # Errors
    ///
    /// Returns [`PraxisError::MalformedCredentials`] only if a discovered
    /// file exists but cannot be read or parsed.
    pub fn discover() -> Result<Self, PraxisError> {
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
        if let Ok(p) = std::env::var("PRAXIS_CONFIG") {
            paths.push(std::path::PathBuf::from(p));
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            paths.push(std::path::PathBuf::from(xdg).join("praxis/credentials.toml"));
        }
        if let Ok(home) = std::env::var("HOME") {
            paths.push(std::path::PathBuf::from(home).join(".config/praxis/credentials.toml"));
        }
        paths
    }

    /// The override block for `name`, if present.
    pub fn override_for(&self, name: &str) -> Option<&ProviderOverride> {
        self.providers.get(name)
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
    pub fn discover_or(path: Option<&std::path::Path>) -> Result<Self, PraxisError> {
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
/// Returns [`PraxisError::IncompleteCustomProvider`] if a custom provider is
/// missing a required field.
pub fn resolve_configs(
    registry: &[Provider],
    credentials: &Credentials,
    env_keys: &HashMap<String, String>,
) -> Result<Vec<HttpConfig>, PraxisError> {
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
/// Returns [`PraxisError::IncompleteCustomProvider`] if a custom provider is
/// missing a required field.
pub fn resolve_configs_with_keyring(
    registry: &[Provider],
    credentials: &Credentials,
    env_keys: &HashMap<String, String>,
    keyring_keys: &HashMap<String, String>,
) -> Result<Vec<HttpConfig>, PraxisError> {
    let registry_names: std::collections::HashSet<&str> =
        registry.iter().map(|p| p.name()).collect();
    let mut out = Vec::new();

    // 1. Registry providers, possibly overridden by config.
    for reg in registry {
        let cfg = credentials.override_for(reg.name());
        let key_var = reg.key_env_var();
        let api_key = keyring_keys
            .get(key_var)
            .cloned()
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
            return Err(PraxisError::incomplete_custom_provider(name, missing));
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
) -> Result<Vec<HttpConfig>, PraxisError> {
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
    resolve_configs_with_keyring(Provider::registry(), &credentials, &env_keys, &keyring_keys)
}

/// The thin CLI-facing wrapper over [`resolve_configs`]: discovers the
/// credentials config, snapshots the real environment, and resolves against
/// the built-in provider registry.
///
/// Returns an empty `Vec` (not an error) when nothing is authed — surfacing
/// [`PraxisError::NoAuthedProviders`] is the caller's job, since the right
/// response depends on context (the CLI errors; a library caller may proceed).
///
/// # Errors
///
/// Returns [`PraxisError::MalformedCredentials`] if a discovered config file
/// is unreadable or unparseable, or [`PraxisError::IncompleteCustomProvider`]
/// if a custom provider is missing required fields.
pub fn authed_configs() -> Result<Vec<HttpConfig>, PraxisError> {
    authed_configs_with(None)
}

// ---- OS keychain tier (behind the `keyring` feature) ----

/// The keychain service name Praxis stores keys under. An entry for a
/// provider is looked up as `praxis:<its key env var>` (e.g. `praxis:DEEPSEEK_API_KEY`).
#[cfg(feature = "keyring")]
pub const KEYRING_SERVICE: &str = "praxis";

/// Reads one provider's key from the OS keychain, keyed by its env-var name.
///
/// Returns `Ok(Some(key))` if an entry exists under `praxis:<key_env_var>`,
/// `Ok(None)` if no entry exists, or `Err` if the keychain itself is
/// inaccessible (no Secret Service on a headless Linux box, etc.). A missing
/// entry is the normal case (most providers won't have one); a keychain error
/// is logged-but-skipped at the caller so a broken keychain doesn't sink the
/// whole run.
///
/// # Errors
///
/// Returns [`PraxisError::KeyringAccess`] if the keychain backend errors.
#[cfg(feature = "keyring")]
pub fn read_keyring(key_env_var: &str) -> Result<Option<String>, PraxisError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, key_env_var)
        .map_err(|e| PraxisError::keyring_access(key_env_var, e))?;
    match entry.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(PraxisError::keyring_access(key_env_var, e)),
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
