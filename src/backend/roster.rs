// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! The multi-provider roster: a registry of OpenAI-compatible provider
//! presets and a pure, seeded random-assignment function that pairs critic
//! personas with authed providers.
//!
//! Model diversity improves cross-examination — different frontier models
//! have different blind spots — so the roster assigns each critic persona a
//! provider drawn (pseudo-)randomly from the set whose API keys are present
//! in the environment. The assignment is a **pure** function of
//! `(personas, authed_configs, rng)`, so it is fully deterministic given the
//! RNG state and unit-testable without touching the environment.

use crate::backend::http::HttpConfig;
use crate::persona::Persona;

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
    /// use proserpina::backend::roster::Provider;
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
    /// use proserpina::backend::roster::Provider;
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
                    .with_model("gpt-4o")
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

/// Builds a critic roster by randomly assigning each persona one of the
/// provided `configs`.
///
/// For each persona (in order), independently picks a uniformly random config
/// from `configs` and pairs it with a clone of the persona. Returns one entry
/// per persona; empty if either input is empty. Two critics may share a
/// model — persona- plus model-diversity together is the goal, not model
/// uniqueness.
///
/// This function is **pure**: it does not touch the environment. Deterministic
/// given the RNG state, so it is fully unit-testable with hand-built configs
/// and a seeded RNG. To build a roster from a provider registry, see
/// [`roster_from_env`].
///
/// # Examples
///
/// ```
/// use proserpina::backend::http::HttpConfig;
/// use proserpina::backend::roster::random_roster;
/// use proserpina::Persona;
/// use rand::SeedableRng;
/// use rand::rngs::StdRng;
///
/// let configs = vec![HttpConfig {
///     base_url: "https://example.invalid/v1".to_owned(),
///     model: "m".to_owned(),
///     api_key: "k".to_owned(),
/// }];
/// let personas = vec![Persona::new("a"), Persona::new("b")];
/// let roster = random_roster(&personas, &configs, &mut StdRng::seed_from_u64(1));
/// assert_eq!(roster.len(), 2);
/// ```
pub fn random_roster(
    personas: &[Persona],
    configs: &[HttpConfig],
    rng: &mut impl rand::Rng,
) -> Vec<(Persona, HttpConfig)> {
    if configs.is_empty() || personas.is_empty() {
        return Vec::new();
    }
    personas
        .iter()
        .map(|persona| {
            let idx = rng.random_range(0..configs.len());
            (persona.clone(), configs[idx].clone())
        })
        .collect()
}

/// Builds a roster by reading keys for `providers` from the environment,
/// keeping only the authed ones, and randomly assigning them to `personas`
/// with an RNG seeded from `seed`.
///
/// This is the CLI-friendly entry point: it composes [`Provider::config_from_env`]
/// with [`random_roster`]. The underlying `random_roster` stays pure and
/// unit-testable; this function owns the env-reading and seeding.
///
/// # Errors
///
/// Returns [`crate::ProserpinaError::NoAuthedProviders`] when none of `providers`
/// have their key env var set.
pub fn roster_from_env(
    personas: &[Persona],
    providers: &[Provider],
    seed: u64,
) -> Result<Vec<(Persona, HttpConfig)>, crate::ProserpinaError> {
    use rand::SeedableRng;

    let configs: Vec<HttpConfig> = providers
        .iter()
        .filter_map(|p| p.config_from_env())
        .collect();

    if configs.is_empty() {
        return Err(crate::ProserpinaError::no_authed_providers(
            providers.iter().map(|p| p.name().to_owned()).collect(),
        ));
    }

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    Ok(random_roster(personas, &configs, &mut rng))
}
