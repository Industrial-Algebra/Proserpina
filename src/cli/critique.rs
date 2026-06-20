// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `praxis critique` subcommand and its library entry point.

use crate::agent::AgentId;
use crate::backend::EchoAgent;
use crate::error::PraxisError;
use crate::graph::{InteractionGraph, Topology};
use crate::persona::Persona;
use crate::report::Report;
use crate::runner::Runner;
use crate::subject::Subject;

/// The default critic panel used when none is configured.
///
/// v1 ships a single Devil's Advocate persona behind the echo backend. Real
/// backends and configurable panels land later; this keeps the CLI runnable.
fn default_panel() -> Vec<(AgentId, Persona)> {
    vec![(
        AgentId::new("devils-advocate"),
        Persona::new("Devil's Advocate")
            .with_framing("Assume the proposal is wrong; find how.")
            .with_focus("logical gaps and unsupported assumptions"),
    )]
}

/// Runs a critique over the given subject text and returns the markdown report.
///
/// Builds a default echo-backed parallel panel, executes it over the subject,
/// synthesizes the transcript into a report, and renders it as markdown. This
/// is the testable core of `praxis critique`; the binary wraps it with file
/// I/O.
///
/// # Errors
///
/// Returns [`PraxisError`] if the run fails (the echo backend never does, but
/// real backends will).
pub fn run_critique(input: &str, source: &str) -> Result<String, PraxisError> {
    let panel = default_panel();
    let critic_ids: Vec<AgentId> = panel.iter().map(|(id, _)| id.clone()).collect();
    let graph = InteractionGraph::from(Topology::parallel(critic_ids));

    let mut runner = Runner::new(graph);
    for (id, persona) in panel {
        runner = runner.with_agent(EchoAgent::new(id, persona));
    }

    let subject = Subject::from_markdown(input, source);
    let transcript = runner.execute(&subject)?;
    let report = Report::from_transcript(&transcript);

    Ok(report.to_markdown_with_source(subject.source()))
}
