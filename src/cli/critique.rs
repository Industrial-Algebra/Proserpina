// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! The `proserpina critique` subcommand and its library entry points.
//!
//! Two entry points:
//! - [`run_critique_echo`] — the offline, deterministic echo-backed path.
//!   Always available under the `cli` feature; used for testing and for
//!   `--echo` runs.
//! - [`run_critique`] — the multi-provider roster path (requires `cli` +
//!   `backend-http`). Assigns authed frontier providers to critic personas at
//!   random (seeded), runs them as HTTP agents, and renders the report. The
//!   default CLI path when `backend-http` is compiled in.

use crate::agent::Agent;
use crate::agent::AgentId;
use crate::backend::http::HttpConfig;
use crate::backend::EchoAgent;
use crate::error::ProserpinaError;
use crate::graph::{InteractionGraph, Topology};
use crate::persona::Persona;
use crate::report::Report;
use crate::runner::Runner;
use crate::subject::Subject;
use crate::transcript::Transcript;

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
/// Returns [`ProserpinaError`] if the run fails (the echo backend never does).
pub fn run_critique_echo(input: &str, source: &str) -> Result<String, ProserpinaError> {
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
/// Returns [`ProserpinaError::NoAuthedProviders`] when no provider key is set in
/// the environment. Returns [`ProserpinaError::AgentFailure`] if a provider fails
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
) -> Result<String, ProserpinaError> {
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
        return Err(ProserpinaError::no_authed_providers(
            Provider::registry()
                .iter()
                .map(|p| p.name().to_owned())
                .collect(),
        ));
    }

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let roster = random_roster(&personas, &configs, &mut rng);

    let subject = Subject::from_markdown(input, source);
    let system = AgentId::new(crate::runner::SYSTEM_AGENT);

    // Execute critics with graceful degradation: if a critic's provider fails
    // (after retries), try reassigning to a different authed config. If all
    // configs fail for a persona, skip it and note the skip.
    let mut transcript = Transcript::new();
    let mut skipped: Vec<String> = Vec::new();

    for (persona, primary_config) in &roster {
        let persona_name = persona.name();
        let id = AgentId::new(persona_name);

        // Build the list of configs to try: primary first, then the others.
        let mut configs_to_try: Vec<&HttpConfig> = vec![primary_config];
        for c in &configs {
            if c.model != primary_config.model {
                configs_to_try.push(c);
            }
        }

        let prompt = crate::message::Message::new(
            system.clone(),
            Some(id.clone()),
            crate::message::MessageKind::Prompt,
            subject.text().to_owned(),
        );

        let mut succeeded = false;
        for (attempt_idx, cfg) in configs_to_try.iter().enumerate() {
            let mut agent =
                HttpAgent::new_with_policy(id.clone(), persona.clone(), (*cfg).clone(), policy);
            match agent.respond(&prompt) {
                Ok(response) => {
                    if !json && attempt_idx > 0 {
                        eprintln!("  ✓ {persona_name} (reassigned to {})", cfg.model);
                    } else if !json {
                        eprintln!("  ✓ {persona_name} ({})", cfg.model);
                    }
                    transcript.push(response);
                    succeeded = true;
                    break;
                }
                Err(e) => {
                    if !json {
                        eprintln!(
                            "  ✗ {persona_name} ({}) failed, trying next provider...",
                            cfg.model
                        );
                    }
                    let _ = e; // error is surfaced via stderr above
                }
            }
        }

        if !succeeded {
            if !json {
                eprintln!("  — {persona_name} skipped (all providers failed)");
            }
            skipped.push(persona_name.to_owned());
        }
    }

    // If ALL critics failed, there's nothing to summarize.
    if transcript.is_empty() {
        return Err(ProserpinaError::agent_failure(
            "all critics",
            "every provider failed for every critic; nothing to report",
        ));
    }

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
        if !skipped.is_empty() {
            body.push_str(&format!(
                "\n_Skipped critics (provider failures): {}_\n",
                skipped.join(", ")
            ));
        }
    }
    Ok(body)
}

/// Resolves the roster for a critique and emits a [`Plan`] without making any
/// API calls (`proserpina critique --dry-run`). Lets an agent verify intent before
/// spending tokens.
///
/// # Errors
///
/// Returns [`ProserpinaError::NoAuthedProviders`] when no provider key is set
/// (same as a real run).
#[cfg(all(feature = "cli", feature = "backend-http"))]
pub fn plan_critique(
    _input: &str,
    _source: &str,
    seed: u64,
    config_path: Option<&std::path::Path>,
    _json: bool,
    panel: Option<&str>,
) -> Result<String, ProserpinaError> {
    use crate::agent_info::Plan;
    use crate::backend::credentials::{authed_configs_with, Credentials};
    use crate::backend::roster::Provider;
    use crate::persona::resolve_panel;

    let credentials = Credentials::discover_or(config_path)?;
    let configs = authed_configs_with(config_path)?;
    if configs.is_empty() {
        return Err(ProserpinaError::no_authed_providers(
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
