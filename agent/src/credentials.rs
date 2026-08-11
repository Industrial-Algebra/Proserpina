// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Credential-resolution core: the provider registry, the parsed
//! `credentials.toml` model, and the pure resolution functions.
//!
//! This module is the daemon-friendly surface (Ijima's use case): given an
//! explicit registry, a [`Credentials`] value, and explicit key snapshots, it
//! resolves effective [`HttpConfig`]s with **no filesystem, environment,
//! keychain, or HTTP-client dependencies**. The convenience layers that DO
//! touch the environment (env-reading wrappers, pi provider discovery, the OS
//! keychain) live in `proserpina`'s credentials module and build on this core.
//!
//! Precedence for each resolved provider: OAuth access token > keyring
//! snapshot > env snapshot > config `api_key`; `model`/`base_url`: config
//! override > registry default.

use std::collections::HashMap;
use std::path::Path;

use crate::error::ProserpinaError;

/// A preset for an OpenAI-compatible provider: where to call, which model to
/// request, and which environment variable holds the API key.
///
/// `Provider` is data, not an enum — adding providers is data, not code. A
/// built-in registry ([`Provider::registry`]) ships the common frontier
/// presets; users can construct their own.
#[derive(Debug, Clone)]
pub struct Provider {
    name: String,
    base_url: String,
    model: String,
    key_env_var: String,
}

impl Provider {
    /// Creates a provider preset with the given short name; set the remaining
    /// fields with the `with_*` builders.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina_agent::credentials::Provider;
    /// let p = Provider::new("deepseek")
    ///     .with_base_url("https://api.deepseek.com/v1")
    ///     .with_model("deepseek-chat")
    ///     .with_key_env_var("DEEPSEEK_API_KEY");
    /// assert_eq!(p.name(), "deepseek");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_url: String::new(),
            model: String::new(),
            key_env_var: String::new(),
        }
    }

    /// Sets the API base URL (without `/chat/completions`).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets the model to request.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets the name of the environment variable holding the API key.
    #[must_use]
    pub fn with_key_env_var(mut self, key_env_var: impl Into<String>) -> Self {
        self.key_env_var = key_env_var.into();
        self
    }

    /// The provider's short name (e.g. `deepseek`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The API base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The model to request.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The environment variable holding the API key.
    pub fn key_env_var(&self) -> &str {
        &self.key_env_var
    }

    /// Reads the API key from the environment and builds an [`HttpConfig`].
    ///
    /// Returns `None` if the key is unset, so the caller can filter a registry
    /// down to the providers that are actually authed.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina_agent::credentials::Provider;
    /// let p = Provider::new("test")
    ///     .with_base_url("https://example.invalid/v1")
    ///     .with_model("m")
    ///     .with_key_env_var("PROSERPINA_DOC_EXAMPLE_UNSET");
    /// // Key unset -> None.
    /// assert!(p.config_from_env().is_none());
    /// ```
    pub fn config_from_env(&self) -> Option<HttpConfig> {
        let api_key = std::env::var(&self.key_env_var).ok()?;
        Some(HttpConfig {
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            api_key,
        })
    }

    /// The built-in registry of frontier OpenAI-compatible providers.
    ///
    /// Ships sensible defaults for DeepSeek, OpenAI, Moonshot, Alibaba,
    /// Z.ai, and Google. Model strings drift over time; treat these as
    /// defaults and override via [`Provider::new`] when you need a specific
    /// model. Every entry is fully populated (non-empty base URL, model, and
    /// key env var).
    pub fn registry() -> &'static [Provider] {
        static REGISTRY: std::sync::OnceLock<Vec<Provider>> = std::sync::OnceLock::new();
        REGISTRY.get_or_init(|| {
            vec![
                Provider::new("deepseek")
                    .with_base_url("https://api.deepseek.com/v1")
                    .with_model("deepseek-chat")
                    .with_key_env_var("DEEPSEEK_API_KEY"),
                Provider::new("openai")
                    .with_base_url("https://api.openai.com/v1")
                    .with_model("gpt-5.4")
                    .with_key_env_var("OPENAI_API_KEY"),
                Provider::new("moonshot")
                    .with_base_url("https://api.moonshot.cn/v1")
                    .with_model("moonshot-v1-auto")
                    .with_key_env_var("MOONSHOT_API_KEY"),
                Provider::new("alibaba")
                    .with_base_url("https://dashscope.aliyuncs.com/compatible-mode/v1")
                    .with_model("qwen-plus")
                    .with_key_env_var("DASHSCOPE_API_KEY"),
                Provider::new("zai")
                    .with_base_url("https://api.z.ai/api/coding/paas/v4")
                    .with_model("glm-5.2")
                    .with_key_env_var("ZAI_API_KEY"),
                Provider::new("google")
                    .with_base_url("https://generativelanguage.googleapis.com/v1beta/openai")
                    .with_model("gemini-1.5-pro")
                    .with_key_env_var("GOOGLE_API_KEY"),
            ]
        })
    }
}

/// Configuration for an [`HttpAgent`]: where to call and how to authenticate.
///
/// Works with any OpenAI-compatible chat-completions endpoint.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// The base URL of the API (without `/chat/completions`). For DeepSeek:
    /// `https://api.deepseek.com/v1`.
    pub base_url: String,
    /// The model to request, e.g. `deepseek-chat`, `gpt-4o-mini`.
    pub model: String,
    /// The API key. Read from the environment (e.g. `DEEPSEEK_API_KEY`) at
    /// call sites, not hard-coded.
    pub api_key: String,
}

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

/// One persona in a config-defined panel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct PersonaSpec {
    pub name: String,
    pub framing: Option<String>,
    pub focus: Option<String>,
}

impl PersonaSpec {
    /// Converts this spec into a [`Persona`].
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

/// The `[retry]` section of a credentials config file. All fields optional;
/// missing fields fall back to [`RetryPolicy::DEFAULT`] at resolution time.
///
/// Lives here (not in the credentials module) because [`RetryPolicy::resolve`]
/// consumes it; the `proserpina` credentials module re-exports it so the
/// public path `proserpina::backend::credentials::RetryConfig` is preserved.
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
    /// use proserpina_agent::credentials::Credentials;
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

///
/// A custom provider `my-local-llm` falls back to `MY_LOCAL_LLM_API_KEY`.
pub fn env_var_name_for(provider_name: &str) -> String {
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

/// Extracts the hostname from a base_url for provider-dedup purposes.
/// `https://api.deepseek.com/v1` → `api.deepseek.com`
/// `https://api.deepseek.com` → `api.deepseek.com`
pub fn extract_host(url: &str) -> String {
    let no_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let no_port = no_scheme.split(':').next().unwrap_or(no_scheme);
    let no_path = no_port.split('/').next().unwrap_or(no_port);
    no_path.to_owned()
}
