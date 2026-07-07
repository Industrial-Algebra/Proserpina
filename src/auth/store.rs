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
        // Store OAuth as a custom-format override with the access token as the api_key.
        // The refresh token + expiry go in extra fields that the resolution layer
        // reads for refresh.
        // For now: store the access token as api_key; refresh logic reads it.
        // TODO: extend ProviderOverride or Credentials to carry OAuth refresh+expires.
        self.credentials.set_override(
            provider_name,
            ProviderOverride {
                api_key: Some(access.to_owned()),
                model: None,
                base_url: None,
            },
        );
        let _ = (refresh, expires_ms); // will be stored when ProviderOverride is extended
        self.save()
    }

    /// Removes a provider's credentials.
    pub fn remove(&mut self, provider_name: &str) -> Result<(), ProserpinaError> {
        self.credentials.remove_override(provider_name);
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
        // For now, this is a no-op stub — the OAuth token storage format needs
        // to be extended in ProviderOverride to carry refresh+expires fields.
        // The infrastructure (oauth::refresh_token) is ready; the wiring depends
        // on extending the credentials format.
        //
        // TODO for full v0.3.0: extend ProviderOverride with OAuth token fields
        // (access, refresh, expires) and implement the refresh loop here.
        Ok(())
    }
}
