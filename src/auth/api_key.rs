// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0-only

//! API-key auth flow: prompt, validate, store.

use crate::backend::http::{validate_provider, RetryPolicy};
use crate::ProserpinaError;

use super::{auth_to_http_config, AuthMethod, AuthUi, ProviderAuth};

/// Runs the API-key auth flow for a provider: prompt for a key, validate it,
/// return the validated key for storage.
///
/// # Errors
///
/// Returns [`ProserpinaError`] if validation fails (the key is rejected by
/// the provider).
pub fn run_api_key_flow(
    ui: &dyn AuthUi,
    provider: &ProviderAuth,
) -> Result<String, ProserpinaError> {
    let AuthMethod::ApiKey { signup_url, .. } = &provider.method else {
        return Err(ProserpinaError::agent_failure(
            provider.name,
            "not an API-key provider",
        ));
    };

    let key = ui.prompt_api_key(provider.name, signup_url)?;

    ui.show_status("Validating...");
    let config = auth_to_http_config(provider, key.clone());
    validate_provider(&config, &RetryPolicy::NONE)?;

    Ok(key)
}
