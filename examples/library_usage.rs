// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Library-usage example: build a Proserpina run programmatically, no CLI, no
//! network. Uses the deterministic echo backend so it runs anywhere with no
//! API keys. Demonstrates Proserpina as an embedded critique engine.
//!
//! Run with:
//!   cargo run --example library_usage
//!
//! (No features required — the echo backend is always available.)

use proserpina::{
    AgentId, EchoAgent, Finding, InteractionGraph, Persona, Report, Runner, Severity, Subject,
    Topology,
};

fn main() {
    // 1. Define the panel (two critics).
    let personas = vec![
        Persona::new("Devil's Advocate").with_framing("Assume the proposal is wrong; find how."),
        Persona::new("Methodologist").with_framing("Scrutinize rigor."),
    ];

    // 2. Build a parallel-topology graph over their ids.
    let critics: Vec<AgentId> = personas.iter().map(|p| AgentId::new(p.name())).collect();
    let graph = InteractionGraph::from(Topology::parallel(critics));

    // 3. Register an echo agent per persona.
    let mut runner = Runner::new(graph);
    for persona in personas {
        let id = AgentId::new(persona.name());
        runner = runner.with_agent(EchoAgent::new(id, persona));
    }

    // 4. Run over a subject.
    let subject = Subject::from_markdown(
        "# Plan\n\nWe will ship a distributed database with no consistency model.",
        "plan.md",
    );
    let transcript = runner.execute(&subject).expect("echo run never fails");

    // 5. Synthesize a report. With the echo backend we use the simple
    //    from_transcript fold (one Finding per Critique). A real run would
    //    call the summarizer for clustered rich findings.
    let report = Report::from_transcript(&transcript);

    println!("=== transcript ({} messages) ===", transcript.len());
    for msg in transcript.iter() {
        println!("  [{}] {:?}", msg.sender(), msg.kind());
    }

    println!("\n=== report ({} findings) ===", report.findings().len());
    let markdown = report.to_markdown_with_source(subject.source());
    print!("{markdown}");

    // 6. Or build findings by hand and render JSON-style (illustrative).
    let mut custom = Report::new();
    custom.push_finding(
        Finding::new(Severity::Blocker, "No consistency model defined.")
            .with_category("consistency")
            .with_location("§1"),
    );
    println!("\n=== custom report ===\n{}", custom.to_markdown());
}
