// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0-only

//! OAuth PKCE flow for OpenAI Codex (ChatGPT).
//!
//! Implements the same PKCE flow as OpenAI's official Codex CLI and pi:
//! generate a code verifier/challenge, open a browser for authorization,
//! run a local callback server, exchange the authorization code for tokens.

use crate::ProserpinaError;
use base64::Engine;
use sha2::{Digest, Sha256};

/// The OAuth token result from a successful exchange.
pub struct OAuthTokens {
    pub access: String,
    pub refresh: String,
    pub expires_ms: u64, // millisecond epoch
}

/// Generates a PKCE code verifier (43-128 char random string) and its S256
/// challenge.
pub fn generate_pkce() -> (String, String) {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);

    (verifier, challenge)
}

/// Builds the authorization URL for the browser.
pub fn build_authorize_url(
    client_id: &str,
    authorize_url: &str,
    redirect_uri: &str,
    scope: &str,
    challenge: &str,
    state: &str,
) -> String {
    format!(
        "{authorize_url}?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}\
         &scope={scope}&code_challenge={challenge}&code_challenge_method=S256&state={state}\
         &codex_cli_simplified_flow=true&originator=proserpina"
    )
}

/// Exchanges an authorization code for tokens.
///
/// # Errors
///
/// Returns [`ProserpinaError`] if the exchange fails.
pub fn exchange_code(
    token_url: &str,
    client_id: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<OAuthTokens, ProserpinaError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ProserpinaError::agent_failure("oauth", format!("runtime: {e}")))?;

    runtime.block_on(async {
        let client = reqwest::Client::new();
        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
        ];

        let resp = client
            .post(token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| ProserpinaError::agent_failure("oauth", format!("exchange: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProserpinaError::agent_failure(
                "oauth",
                format!("token exchange HTTP {status}: {body}"),
            ));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProserpinaError::agent_failure("oauth", format!("parse: {e}")))?;

        let access = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProserpinaError::agent_failure("oauth", "no access_token"))?;
        let refresh = json
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProserpinaError::agent_failure("oauth", "no refresh_token"))?;
        let expires_in = json
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        let expires_ms = now_ms() + expires_in * 1000;

        Ok(OAuthTokens {
            access: access.to_owned(),
            refresh: refresh.to_owned(),
            expires_ms,
        })
    })
}

/// Refreshes an expired access token using the refresh token.
///
/// # Errors
///
/// Returns [`ProserpinaError`] if the refresh fails.
pub fn refresh_token(
    token_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<OAuthTokens, ProserpinaError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ProserpinaError::agent_failure("oauth", format!("runtime: {e}")))?;

    runtime.block_on(async {
        let client = reqwest::Client::new();
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
        ];

        let resp = client
            .post(token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| ProserpinaError::agent_failure("oauth", format!("refresh: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProserpinaError::agent_failure(
                "oauth",
                format!("token refresh HTTP {status}: {body}"),
            ));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProserpinaError::agent_failure("oauth", format!("parse: {e}")))?;

        let access = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProserpinaError::agent_failure("oauth", "no access_token"))?;
        let refresh = json
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProserpinaError::agent_failure("oauth", "no refresh_token"))?;
        let expires_in = json
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        let expires_ms = now_ms() + expires_in * 1000;

        Ok(OAuthTokens {
            access: access.to_owned(),
            refresh: refresh.to_owned(),
            expires_ms,
        })
    })
}

/// Current time in milliseconds since Unix epoch.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Generates a random hex state string for CSRF protection.
pub fn generate_state() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
