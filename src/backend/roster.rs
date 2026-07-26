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

/// Re-exported from `proserpina-agent`: the provider preset type and the
/// built-in registry ([`Provider::registry`]). The roster-assignment
/// functions below are the critique-pipeline layer that consumes it.
pub use proserpina_agent::credentials::Provider;

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
