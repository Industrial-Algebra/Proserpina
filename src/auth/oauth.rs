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

/// The local callback server port (must match the redirect_uri).
pub const CALLBACK_PORT: u16 = 1455;
pub const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";

/// Starts a local HTTP server on port 1455 and waits for the OAuth callback.
/// Returns the authorization code from the callback query string.
///
/// The server validates the `state` parameter (CSRF protection) and extracts
/// the `code` parameter. It serves a simple HTML page to the browser so the
/// user sees a success/failure message.
///
/// # Errors
///
/// Returns [`ProserpinaError`] if the server can't start, times out (120s),
/// or the callback has a mismatched state.
pub fn wait_for_callback(expected_state: &str) -> Result<String, ProserpinaError> {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    let listener = TcpListener::bind(format!("127.0.0.1:{CALLBACK_PORT}"))
        .map_err(|e| ProserpinaError::agent_failure("oauth", format!("callback bind: {e}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| ProserpinaError::agent_failure("oauth", format!("nonblocking: {e}")))?;

    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(120);

    loop {
        if start.elapsed() > timeout {
            return Err(ProserpinaError::agent_failure(
                "oauth",
                "callback timed out (120s)",
            ));
        }

        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);

                // Parse the request line: GET /auth/callback?code=xxx&state=yyy HTTP/1.1
                let request_line = request.lines().next().unwrap_or("");
                let url = request_line.split_whitespace().nth(1).unwrap_or("");

                let (code, error_html) = parse_callback(url, expected_state);

                let html = if let Some(ref err) = error_html {
                    err.clone()
                } else {
                    SUCCESS_HTML.to_owned()
                };

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    html.len(),
                    html
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();

                if let Some(code) = code {
                    return Ok(code);
                }
                // If error_html, the callback had an error — keep waiting.
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(ProserpinaError::agent_failure(
                    "oauth",
                    format!("callback accept: {e}"),
                ));
            }
        }
    }
}

/// Parses the callback URL, extracting the code and validating state.
/// Returns (code, optional_error_html).
fn parse_callback(url: &str, expected_state: &str) -> (Option<String>, Option<String>) {
    // Expected: /auth/callback?code=xxx&state=yyy
    let query = url.split('?').nth(1).unwrap_or("");
    let params: std::collections::HashMap<&str, &str> = query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .collect();

    // Check for error response.
    if let Some(err) = params.get("error") {
        return (
            None,
            Some(format!(
                "<html><body><h2>Authentication failed: {}</h2><p>You can close this window.</p></body></html>",
                err
            )),
        );
    }

    // Validate state.
    let state = params.get("state").copied().unwrap_or("");
    if state != expected_state {
        return (
            None,
            Some(
                "<html><body><h2>State mismatch — possible CSRF attack.</h2></body></html>"
                    .to_owned(),
            ),
        );
    }

    // Extract code.
    let code = params.get("code").map(|c| c.to_string());
    let has_code = code.is_some();
    (
        code,
        if has_code {
            None
        } else {
            Some("<html><body><h2>Missing authorization code.</h2></body></html>".to_owned())
        },
    )
}

const SUCCESS_HTML: &str = "<html><body><h2>✓ Proserpina authenticated successfully!</h2><p>You can close this window.</p></body></html>";

/// Runs the complete OAuth flow: PKCE → browser → callback → token exchange.
/// Returns the resulting tokens.
///
/// # Errors
///
/// Returns [`ProserpinaError`] at any step.
pub fn run_oauth_flow(
    client_id: &str,
    authorize_url: &str,
    token_url: &str,
    scope: &str,
) -> Result<OAuthTokens, ProserpinaError> {
    // 1. Generate PKCE challenge + state.
    let (verifier, challenge) = generate_pkce();
    let state = generate_state();

    // 2. Build the authorization URL.
    let url = build_authorize_url(
        client_id,
        authorize_url,
        REDIRECT_URI,
        scope,
        &challenge,
        &state,
    );

    // 3. Open the browser.
    let _ = webbrowser::open(&url);

    // 4. Wait for the callback.
    let code = wait_for_callback(&state)?;

    // 5. Exchange the code for tokens.
    exchange_code(token_url, client_id, &code, &verifier, REDIRECT_URI)
}
