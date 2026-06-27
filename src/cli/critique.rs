// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `praxis critique` subcommand and its library entry points.
//!
//! Two entry points:
//! - [`run_critique_echo`] — the offline, deterministic echo-backed path.
//!   Always available under the `cli` feature; used for testing and for
//!   `--echo` runs.
//! - [`run_critique`] — the multi-provider roster path (requires `cli` +
//!   `backend-http`). Assigns authed frontier providers to critic personas at
//!   random (seeded), runs them as HTTP agents, and renders the report. The
//!   default CLI path when `backend-http` is compiled in.

use crate::agent::AgentId;
use crate::backend::EchoAgent;
use crate::error::PraxisError;
use crate::graph::{InteractionGraph, Topology};
use crate::persona::Persona;
use crate::report::Report;
use crate::runner::Runner;
use crate::subject::Subject;

/// The default critic personas used when none is configured.
/// Delegates to [`crate::persona::Persona::default_panel`].
pub fn default_personas() -> Vec<Persona> {
    crate::persona::Persona::default_panel()
}

/// Runs a critique using the offline echo backend and returns the markdown
/// report.
///
/// This is the testable, no-network, no-keys entry point. The binary uses it
/// for `--echo` runs; tests use it to exercise the CLI surface without any LLM
/// dependency.
///
/// # Errors
///
/// Returns [`PraxisError`] if the run fails (the echo backend never does).
pub fn run_critique_echo(input: &str, source: &str) -> Result<String, PraxisError> {
    let personas = default_personas();
    // Echo agents use the persona name as their AgentId.
    let critic_ids: Vec<AgentId> = personas.iter().map(|p| AgentId::new(p.name())).collect();
    let graph = InteractionGraph::from(Topology::parallel(critic_ids));

    let mut runner = Runner::new(graph);
    for persona in personas {
        let id = AgentId::new(persona.name());
        runner = runner.with_agent(EchoAgent::new(id, persona));
    }

    let subject = Subject::from_markdown(input, source);
    let transcript = runner.execute(&subject)?;
    let report = Report::from_transcript(&transcript);
    Ok(report.to_markdown_with_source(subject.source()))
}

/// Runs a critique using the multi-provider roster (requires `cli` +
/// `backend-http`).
///
/// Builds authed [`HttpConfig`](crate::backend::http::HttpConfig)s from the
/// provider registry, randomly assigns them to the default critic personas
/// with an RNG seeded from `seed`, runs them as HTTP agents, and renders the
/// report. The seed is recorded in the report header so the run is
/// reproducible.
///
/// # Errors
///
/// Returns [`PraxisError::NoAuthedProviders`] when no provider key is set in
/// the environment. Returns [`PraxisError::AgentFailure`] if a provider fails
/// to respond.
#[cfg(all(feature = "cli", feature = "backend-http"))]
pub fn run_critique(
    input: &str,
    source: &str,
    seed: u64,
    config_path: Option<&std::path::Path>,
    json: bool,
    panel: Option<&str>,
    policy: crate::backend::http::RetryPolicy,
) -> Result<String, PraxisError> {
    use crate::backend::credentials::{authed_configs_with, Credentials};
    use crate::backend::http::HttpAgent;
    use crate::backend::roster::{random_roster, Provider};
    use crate::persona::resolve_panel;
    use crate::summary::summarize;
    use rand::SeedableRng;

    let credentials = Credentials::discover_or(config_path)?;
    let personas = resolve_panel(panel.unwrap_or("default"), &credentials)?;
    let configs = authed_configs_with(config_path)?;
    if configs.is_empty() {
        return Err(PraxisError::no_authed_providers(
            Provider::registry()
                .iter()
                .map(|p| p.name().to_owned())
                .collect(),
        ));
    }

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let roster = random_roster(&personas, &configs, &mut rng);

    let critic_ids: Vec<AgentId> = roster.iter().map(|(p, _)| AgentId::new(p.name())).collect();
    let graph = InteractionGraph::from(Topology::parallel(critic_ids));

    let mut runner = Runner::new(graph);
    for (persona, config) in roster {
        let id = AgentId::new(persona.name());
        runner = runner.with_agent(HttpAgent::new_with_policy(id, persona, config, policy));
    }

    let subject = Subject::from_markdown(input, source);
    let transcript = runner.execute(&subject)?;

    // Summarizer pass: structure the transcript into rich findings via the
    // first authed config. Graceful on empty (yields no findings).
    let findings = summarize(&subject, &transcript, &configs[0], &policy).unwrap_or_default();
    let mut report = Report::new();
    for f in findings {
        report.push_finding(f);
    }

    let mut body = if json {
        #[cfg(feature = "json")]
        {
            report.to_json()
        }
        #[cfg(not(feature = "json"))]
        {
            report.to_markdown_with_source(subject.source())
        }
    } else {
        report.to_markdown_with_source(subject.source())
    };
    if !json {
        // Record the seed on the markdown path so the run is reproducible.
        body.push_str(&format!("\n_Reproducibility: seed `{seed}`_\n"));
    }
    Ok(body)
}

/// Resolves the roster for a critique and emits a [`Plan`] without making any
/// API calls (`praxis critique --dry-run`). Lets an agent verify intent before
/// spending tokens.
///
/// # Errors
///
/// Returns [`PraxisError::NoAuthedProviders`] when no provider key is set
/// (same as a real run).
#[cfg(all(feature = "cli", feature = "backend-http"))]
pub fn plan_critique(
    _input: &str,
    _source: &str,
    seed: u64,
    config_path: Option<&std::path::Path>,
    _json: bool,
    panel: Option<&str>,
) -> Result<String, PraxisError> {
    use crate::agent_info::Plan;
    use crate::backend::credentials::{authed_configs_with, Credentials};
    use crate::backend::roster::Provider;
    use crate::persona::resolve_panel;

    let credentials = Credentials::discover_or(config_path)?;
    let configs = authed_configs_with(config_path)?;
    if configs.is_empty() {
        return Err(PraxisError::no_authed_providers(
            Provider::registry()
                .iter()
                .map(|p| p.name().to_owned())
                .collect(),
        ));
    }
    #[cfg(feature = "json")]
    {
        let personas = resolve_panel(panel.unwrap_or("default"), &credentials)?;
        let plan = Plan::for_parallel(&personas, &configs, seed);
        Ok(serde_json::to_string_pretty(&plan).unwrap_or_else(|_| "{}".to_owned()))
    }
    #[cfg(not(feature = "json"))]
    {
        let _ = panel;
        Ok(format!(
            "Dry-run would use seed {seed} with {} provider config(s).",
            configs.len()
        ))
    }
}
