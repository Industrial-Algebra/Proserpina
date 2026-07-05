// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0-only

//! ratatui implementation of the [`AuthUi`] trait.
//!
//! This is the ONLY file in the auth subsystem that imports ratatui. When
//! Knopper matures, a `tui_knopper.rs` replaces this file with zero changes
//! to the auth logic.

use crate::ProserpinaError;

use super::{AuthUi, ProviderAuth};

/// A ratatui-based auth UI. Uses crossterm for terminal I/O.
pub struct RatatuiAuthUi;

impl AuthUi for RatatuiAuthUi {
    fn select_provider(&self, providers: &[ProviderAuth]) -> Option<usize> {
        // For v0.3.0: simple stdin-based selection (full ratatui list widget
        // can be added later). The trait seam is in place; this implementation
        // is intentionally simple.
        println!("\nAvailable providers:\n");
        for (i, p) in providers.iter().enumerate() {
            let method = match &p.method {
                super::AuthMethod::ApiKey { .. } => "API key",
                #[cfg(feature = "cli")]
                super::AuthMethod::OAuth { .. } => "OAuth (browser)",
            };
            println!("  {}. {} ({})", i + 1, p.name, method);
        }
        println!("\nEnter number (or q to quit): ");

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }
        let input = input.trim();
        if input.eq_ignore_ascii_case("q") || input.is_empty() {
            return None;
        }
        input.parse::<usize>().ok().and_then(|n| {
            if n >= 1 && n <= providers.len() {
                Some(n - 1)
            } else {
                None
            }
        })
    }

    fn prompt_api_key(
        &self,
        provider_name: &str,
        signup_url: &str,
    ) -> Result<String, ProserpinaError> {
        println!("\n  Enter your {provider_name} API key");
        println!("  (Get one at: {signup_url})");
        print!("  > ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| ProserpinaError::agent_failure(provider_name, format!("stdin: {e}")))?;

        let key = input.trim().to_owned();
        if key.is_empty() {
            return Err(ProserpinaError::agent_failure(
                provider_name,
                "no key entered",
            ));
        }
        Ok(key)
    }

    fn oauth_flow(&self, url: &str) -> Result<(), ProserpinaError> {
        println!("\n  Opening browser for authentication...");
        println!("  If it doesn't open, visit:\n  {url}\n");
        println!("  Waiting for authentication...");

        // Open the browser (best-effort).
        let _ = webbrowser::open(url);

        // The actual OAuth flow (callback server + code exchange) is handled
        // by the caller; this method is just the UI notification.
        Ok(())
    }

    fn show_status(&self, message: &str) {
        println!("  {message}");
    }

    fn show_success(&self, message: &str) {
        println!("  ✓ {message}");
    }

    fn show_error(&self, message: &str) {
        eprintln!("  ✗ {message}");
    }
}
