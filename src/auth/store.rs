// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0-only

//! Credential storage for the auth subsystem.
//!
//! Wraps the existing credentials.toml + keyring resolution with write
//! capabilities (the existing code only reads). `AuthStore` can write new
//! credentials back to the store after `auth login`.

use crate::backend::credentials::{Credentials, ProviderOverride};
use crate::backend::http::HttpConfig;
use crate::ProserpinaError;

use std::path::PathBuf;

/// The auth credential store: read AND write access to credentials.toml + keyring.
pub struct AuthStore {
    /// The path to the credentials file (discovered or explicit).
    path: PathBuf,
    /// The parsed credentials.
    credentials: Credentials,
}

impl AuthStore {
    /// Discovers the default credentials store (creates the file if it
    /// doesn't exist yet).
    ///
    /// # Errors
    ///
    /// Returns [`ProserpinaError`] if the credentials file exists but is
    /// unreadable/malformed.
    pub fn discover() -> Result<Self, ProserpinaError> {
        let path = Self::discover_path();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&path, "# Proserpina credentials\n").map_err(|e| {
                ProserpinaError::malformed_credentials(path.display().to_string(), e)
            })?;
        }
        let credentials = Credentials::from_path(&path)?;
        Ok(Self { path, credentials })
    }

    /// Finds the credentials file path (same discovery as Credentials::discover).
    fn discover_path() -> PathBuf {
        if let Ok(p) = std::env::var("PROSERPINA_CONFIG") {
            return PathBuf::from(p);
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("proserpina/credentials.toml");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config/proserpina/credentials.toml");
        }
        PathBuf::from("credentials.toml")
    }

    /// Stores an API key for a provider in the credentials file.
    pub fn store_api_key(&mut self, provider_name: &str, key: &str) -> Result<(), ProserpinaError> {
        self.credentials.set_override(
            provider_name,
            ProviderOverride {
                api_key: Some(key.to_owned()),
                model: None,
                base_url: None,
                oauth_access: None,
                oauth_refresh: None,
                oauth_expires: None,
            },
        );
        self.save()
    }

    /// Stores OAuth tokens for a provider.
    #[cfg(feature = "cli")]
    pub fn store_oauth(
        &mut self,
        provider_name: &str,
        access: &str,
        refresh: &str,
        expires_ms: u64,
    ) -> Result<(), ProserpinaError> {
        self.credentials.set_override(
            provider_name,
            ProviderOverride {
                api_key: None,
                model: None,
                base_url: None,
                oauth_access: Some(access.to_owned()),
                oauth_refresh: Some(refresh.to_owned()),
                oauth_expires: Some(expires_ms),
            },
        );
        self.save()
    }

    /// Removes a provider's credentials.
    pub fn remove(&mut self, provider_name: &str) -> Result<(), ProserpinaError> {
        self.credentials.remove_override(provider_name);
        self.save()
    }

    /// Stores a model override for a provider (merges into existing entry,
    /// preserving api_key/oauth fields).
    pub fn store_model(&mut self, provider_name: &str, model: &str) -> Result<(), ProserpinaError> {
        self.credentials.merge_model(provider_name, model);
        self.save()
    }

    /// Writes the credentials back to the file.
    fn save(&self) -> Result<(), ProserpinaError> {
        let toml = self.credentials.to_toml_string();
        std::fs::write(&self.path, toml)
            .map_err(|e| ProserpinaError::malformed_credentials(self.path.display().to_string(), e))
    }

    /// Returns the resolved authed configs (delegates to the existing resolution).
    pub fn authed_configs(&self) -> Result<Vec<HttpConfig>, ProserpinaError> {
        crate::backend::credentials::authed_configs_with(None)
    }

    /// Checks all stored OAuth credentials and refreshes any that are expired.
    /// Called at startup (before a run) to ensure tokens are fresh.
    ///
    /// # Errors
    ///
    /// Returns [`ProserpinaError`] if the store can't be read. Individual
    /// refresh failures are logged and skipped (the provider just won't be
    /// authed).
    pub fn refresh_expired_oauth(&mut self) -> Result<(), ProserpinaError> {
        use crate::auth::oauth::refresh_token;
        use crate::auth::{auth_registry, AuthMethod};

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let mut changed = false;

        // Collect provider names that need refresh (can't mutate while iterating).
        let to_refresh: Vec<String> = self
            .credentials
            .iter()
            .filter(|(_, ov)| ov.oauth_expires.is_some_and(|exp| exp <= now_ms))
            .filter(|(_, ov)| ov.oauth_refresh.is_some())
            .map(|(name, _)| name.clone())
            .collect();

        for name in to_refresh {
            let ov = self.credentials.override_for(&name).cloned();
            let Some(ov) = ov else { continue };
            let Some(refresh_tok) = &ov.oauth_refresh else {
                continue;
            };

            // Find the provider's auth config to get client_id + token_url.
            let provider_auth = auth_registry().iter().find(|p| p.name == name);
            let (client_id, token_url) = if let Some(pa) = provider_auth {
                if let AuthMethod::OAuth {
                    client_id,
                    token_url,
                    ..
                } = &pa.method
                {
                    (*client_id, *token_url)
                } else {
                    continue; // not an OAuth provider
                }
            } else {
                continue; // unknown provider
            };

            eprintln!("proserpina: refreshing {name} OAuth token...");
            match refresh_token(token_url, client_id, refresh_tok) {
                Ok(tokens) => {
                    // Update the stored override with fresh tokens.
                    let mut new_ov = ov.clone();
                    new_ov.oauth_access = Some(tokens.access);
                    new_ov.oauth_refresh = Some(tokens.refresh);
                    new_ov.oauth_expires = Some(tokens.expires_ms);
                    self.credentials.set_override(&name, new_ov);
                    changed = true;
                    eprintln!("proserpina: ✓ {name} token refreshed.");
                }
                Err(e) => {
                    eprintln!("proserpina: ✗ {name} token refresh failed: {e}");
                    // Leave the stale token in place; it will fail at call time
                    // and the graceful-degradation logic will handle it.
                }
            }
        }

        if changed {
            self.save()?;
        }
        Ok(())
    }
}
