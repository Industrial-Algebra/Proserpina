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

/// The parsed credentials config: provider name → override block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credentials {
    providers: HashMap<String, ProviderOverride>,
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
        let parsed: HashMap<String, ProviderOverride> =
            ::toml::from_str(toml).map_err(|e| PraxisError::malformed_credentials("<str>", e))?;
        Ok(Self { providers: parsed })
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
    let registry_names: std::collections::HashSet<&str> =
        registry.iter().map(|p| p.name()).collect();
    let mut out = Vec::new();

    // 1. Registry providers, possibly overridden by config.
    for reg in registry {
        let cfg = credentials.override_for(reg.name());
        let api_key = env_keys
            .get(reg.key_env_var())
            .cloned()
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
        let api_key = env_keys.get(&env_var).cloned().or(cfg.api_key.clone());
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
    resolve_configs(Provider::registry(), &credentials, &env_keys)
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
