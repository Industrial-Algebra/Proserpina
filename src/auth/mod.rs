// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0-only

//! The auth subsystem: interactive credential acquisition, storage, and refresh.
//!
//! Proserpina's auth layer lets users obtain, validate, store, and refresh
//! provider credentials from the CLI (`proserpina auth login <provider>`).
//! It supports API-key providers (DeepSeek, Z.ai, DashScope, Google, Moonshot)
//! and OAuth providers (OpenAI Codex / ChatGPT).
//!
//! ## TUI seam
//!
//! The interactive UI is behind the [`AuthUi`] trait. Today, ratatui implements
//! it; when Knopper matures, a Knopper implementation replaces it with zero
//! changes to the auth logic.

use crate::backend::http::HttpConfig;
use crate::ProserpinaError;

pub mod api_key;
#[cfg(feature = "cli")]
pub mod oauth;
pub mod store;
#[cfg(feature = "cli")]
pub mod tui_ratatui;

pub use store::AuthStore;

/// How a provider authenticates.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// API key entered by the user, validated via GET /models.
    ApiKey {
        /// The env var name for this provider's key.
        env_var: &'static str,
        /// Where to sign up for a key.
        signup_url: &'static str,
    },
    /// OAuth PKCE flow (browser-based, token exchange, refresh lifecycle).
    #[cfg(feature = "cli")]
    OAuth {
        /// The OAuth client ID.
        client_id: &'static str,
        /// The authorization endpoint URL.
        authorize_url: &'static str,
        /// The token exchange endpoint URL.
        token_url: &'static str,
        /// The requested scopes.
        scope: &'static str,
    },
}

/// A provider's auth configuration — what method to use and where to connect.
#[derive(Debug, Clone)]
pub struct ProviderAuth {
    /// Provider display name (e.g. "deepseek").
    pub name: &'static str,
    /// The auth method for this provider.
    pub method: AuthMethod,
    /// The base URL for API calls (for validation after auth).
    pub base_url: &'static str,
    /// The default model for this provider.
    pub model: &'static str,
}

/// The auth registry: all providers Proserpina knows how to authenticate.
pub fn auth_registry() -> &'static [ProviderAuth] {
    &[
        ProviderAuth {
            name: "deepseek",
            method: AuthMethod::ApiKey {
                env_var: "DEEPSEEK_API_KEY",
                signup_url: "https://platform.deepseek.com/api_keys",
            },
            base_url: "https://api.deepseek.com",
            model: "deepseek-chat",
        },
        ProviderAuth {
            name: "zai",
            method: AuthMethod::ApiKey {
                env_var: "ZAI_API_KEY",
                signup_url: "https://z.ai/manage-apikey/apikey-list",
            },
            base_url: "https://api.z.ai/api/coding/paas/v4",
            model: "glm-5.2",
        },
        ProviderAuth {
            name: "dashscope",
            method: AuthMethod::ApiKey {
                env_var: "DASHSCOPE_API_KEY",
                signup_url: "https://dashscope.console.aliyun.com/apiKey",
            },
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
            model: "qwen-plus",
        },
        ProviderAuth {
            name: "google",
            method: AuthMethod::ApiKey {
                env_var: "GOOGLE_API_KEY",
                signup_url: "https://aistudio.google.com/app/apikey",
            },
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
            model: "gemini-1.5-pro",
        },
        ProviderAuth {
            name: "moonshot",
            method: AuthMethod::ApiKey {
                env_var: "MOONSHOT_API_KEY",
                signup_url: "https://platform.moonshot.cn/console/api-keys",
            },
            base_url: "https://api.moonshot.cn/v1",
            model: "moonshot-v1-auto",
        },
        #[cfg(feature = "cli")]
        ProviderAuth {
            name: "openai",
            method: AuthMethod::OAuth {
                client_id: "app_EMoamEEZ73f0CkXaXp7hrann",
                authorize_url: "https://auth.openai.com/oauth/authorize",
                token_url: "https://auth.openai.com/oauth/token",
                scope: "openid profile email offline_access",
            },
            base_url: "https://api.openai.com/v1",
            model: "gpt-4o",
        },
    ]
}

/// Finds a provider's auth config by name (case-insensitive).
pub fn find_provider_auth(name: &str) -> Option<&'static ProviderAuth> {
    auth_registry()
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

/// The interactive UI trait. Implemented by ratatui today; Knopper later.
#[cfg(feature = "cli")]
pub trait AuthUi {
    /// Shows a provider selector and returns the chosen index.
    fn select_provider(&self, providers: &[ProviderAuth]) -> Option<usize>;

    /// Prompts for an API key. Returns the entered key.
    fn prompt_api_key(
        &self,
        provider_name: &str,
        signup_url: &str,
    ) -> Result<String, ProserpinaError>;

    /// Opens the browser for OAuth and waits for the callback.
    fn oauth_flow(&self, url: &str) -> Result<(), ProserpinaError>;

    /// Shows a status message (e.g. "Validating...").
    fn show_status(&self, message: &str);

    /// Shows a success message.
    fn show_success(&self, message: &str);

    /// Shows an error message.
    fn show_error(&self, message: &str);
}

/// Converts an `AuthMethod` + resolved key/token into an `HttpConfig`.
pub fn auth_to_http_config(provider: &ProviderAuth, api_key: String) -> HttpConfig {
    HttpConfig {
        base_url: provider.base_url.to_owned(),
        model: provider.model.to_owned(),
        api_key,
    }
}
