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


use crate::backend::http::HttpConfig;
use crate::backend::roster::Provider;
use crate::error::ProserpinaError;

/// Re-exported from `proserpina-agent`'s credential-resolution core: the
/// parsed config model ([`Credentials`], [`ProviderOverride`], [`PanelConfig`],
/// [`PersonaSpec`]) and the pure resolution functions. The items here are
/// daemon-friendly (no env/filesystem/keychain deps); the wrappers below are
/// the convenience layer that touches the environment, pi's configs, and the
/// OS keychain.
pub use proserpina_agent::credentials::{
    Credentials, PanelConfig, PersonaSpec, ProviderOverride, env_var_name_for, extract_host, resolve_configs,
    resolve_configs_with_keyring,
};


/// The `[retry]` section of the config file, re-exported from
/// `proserpina-agent` (where [`RetryPolicy`](crate::backend::http::RetryPolicy)
/// lives). All fields optional; missing fields fall back to
/// [`crate::backend::http::RetryPolicy::DEFAULT`] at resolution time.
pub use proserpina_agent::http::RetryConfig;

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
