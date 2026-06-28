// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Agent-facing self-description: [`Capabilities`] and [`Plan`].
//!
//! These types back the `proserpina capabilities` and `proserpina critique --dry-run`
//! commands, making Proserpina discoverable to AI agents. They serialize to JSON
//! (the default for capabilities, since it's machine-facing).
//!
//! [`Capabilities`] reports dynamic auth state — which providers are currently
//! authed in this environment — so an agent learns what it can actually do
//! right now, not just what Proserpina could do in theory.

use crate::backend::credentials::authed_configs_with;
use crate::backend::roster::Provider;

/// Info about one provider, as reported by [`Capabilities`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderInfo {
    /// The provider's short name (e.g. `deepseek`).
    pub name: String,
    /// The model this provider is configured to use.
    pub model: String,
    /// Whether this provider is currently authed (key in env or config).
    pub authed: bool,
}

/// Proserpina's self-description for AI agents.
///
/// Built via [`Capabilities::static_info`] (registry + static metadata) or
/// [`Capabilities::with_current_auth`] (also resolves which providers are
/// authed right now). Serialize to JSON for the `capabilities` command.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Capabilities {
    /// Crate version (`CARGO_PKG_VERSION`).
    pub version: String,
    /// The subcommands the CLI exposes.
    pub subcommands: Vec<String>,
    /// Output formats the report supports.
    pub output_formats: Vec<String>,
    /// Topologies the runner supports.
    pub topologies: Vec<String>,
    /// Provider registry + per-provider authed state.
    pub providers: Vec<ProviderInfo>,
    /// The panel of critic personas currently in use.
    pub personas: Vec<crate::persona::Persona>,
    /// Available panel names (built-in + config-defined).
    pub panels: Vec<String>,
    /// The exit-code scheme (code -> meaning), so agents can learn it.
    pub exit_codes: std::collections::BTreeMap<u8, &'static str>,
}

impl Capabilities {
    /// Static capabilities: registry + fixed metadata, with all providers
    /// marked unauthed (no env/config read).
    pub fn static_info() -> Self {
        let providers = Provider::registry()
            .iter()
            .map(|p| ProviderInfo {
                name: p.name().to_owned(),
                model: p.model().to_owned(),
                authed: false,
            })
            .collect();
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            subcommands: vec!["critique".to_owned(), "capabilities".to_owned()],
            output_formats: vec!["markdown".to_owned(), "json".to_owned()],
            topologies: vec!["parallel".to_owned(), "rounds".to_owned()],
            providers,
            personas: crate::persona::Persona::default_panel(),
            panels: vec!["default".to_owned(), "duo".to_owned(), "panel".to_owned()],
            exit_codes: crate::error::exit_codes_map(),
        }
    }

    /// Capabilities with dynamic auth state: resolves which providers are
    /// currently authed via the credentials config + environment.
    pub fn with_current_auth() -> Self {
        let authed = authed_configs_with(None).unwrap_or_default();
        let authed_models: std::collections::HashSet<&str> =
            authed.iter().map(|c| c.model.as_str()).collect();
        let mut caps = Self::static_info();
        // Mark authed by matching model+name; fall back to model-only match
        // for custom providers not in the registry.
        for p in &mut caps.providers {
            if authed_models.contains(p.model.as_str()) {
                p.authed = true;
            }
        }
        // Custom providers (authed but not in the registry) get appended.
        let registry_names: std::collections::HashSet<&str> =
            Provider::registry().iter().map(|p| p.name()).collect();
        // We approximate custom-provider reporting by surfacing any authed
        // config whose model didn't match a registry entry. (Full name-level
        // resolution would require credentials introspection; deferred.)
        for cfg in &authed {
            let in_registry = caps.providers.iter().any(|p| p.model == cfg.model);
            if !in_registry {
                caps.providers.push(ProviderInfo {
                    name: "custom".to_owned(),
                    model: cfg.model.clone(),
                    authed: true,
                });
            }
            let _ = registry_names;
        }
        // Surface user-defined panels from the config (in addition to built-ins).
        if let Ok(creds) = crate::backend::credentials::Credentials::discover() {
            for name in creds.panels().keys() {
                caps.panels.push(name.clone());
            }
        }
        caps
    }
}

/// A resolved plan for a run, emitted by `proserpina critique --dry-run`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Plan {
    /// The seed that will be (or was) used.
    pub seed: u64,
    /// The topology name.
    pub topology: String,
    /// One entry per critic persona: which provider backs it.
    pub roster: Vec<PlanSlot>,
    /// How many critic LLM calls the run will make.
    pub n_critic_calls: usize,
    /// How many summarizer LLM calls (always 1 in v1).
    pub n_summarizer_calls: usize,
    /// Total LLM calls: critics + summarizer.
    pub estimated_total_calls: usize,
}

/// One slot in a [`Plan`] roster.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanSlot {
    /// The persona backing this slot.
    pub persona: String,
    /// The provider name (or "custom") backing this slot.
    pub provider: String,
    /// The model the provider will use.
    pub model: String,
}

impl Plan {
    /// Builds a plan for a parallel-topology run, resolving the roster with a
    /// seeded RNG **without making any network calls**.
    ///
    /// `configs` are the authed HTTP configs (already resolved from
    /// credentials + env). The provider label for each slot is derived by
    /// matching the config's model against the registry (falling back to
    /// `"custom"`).
    pub fn for_parallel(
        personas: &[crate::persona::Persona],
        configs: &[crate::backend::http::HttpConfig],
        seed: u64,
    ) -> Self {
        use crate::backend::roster::{random_roster, Provider};
        use rand::SeedableRng;

        // Build (model -> provider-name) lookup from the registry.
        let model_to_name: std::collections::HashMap<&str, &str> = Provider::registry()
            .iter()
            .map(|p| (p.model(), p.name()))
            .collect();

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let roster = random_roster(personas, configs, &mut rng);

        let slots = roster
            .iter()
            .map(|(persona, cfg)| {
                let provider = model_to_name
                    .get(cfg.model.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "custom".to_owned());
                PlanSlot {
                    persona: persona.name().to_owned(),
                    provider,
                    model: cfg.model.clone(),
                }
            })
            .collect();

        let n_critic_calls = roster.len();
        Self {
            seed,
            topology: "parallel".to_owned(),
            roster: slots,
            n_critic_calls,
            n_summarizer_calls: 1,
            estimated_total_calls: n_critic_calls + 1,
        }
    }
}
