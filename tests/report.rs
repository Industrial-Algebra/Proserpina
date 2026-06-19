// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the report synthesizer and markdown rendering.

use praxis::{
    AgentId, EchoAgent, InteractionGraph, Message, MessageKind, Persona, Report, Runner, Severity,
    Subject, Topology, Transcript,
};

#[test]
fn severity_is_exhaustive_with_four_levels() {
    // Severity must cover exactly these four levels; a change is conscious.
    let levels = [
        Severity::Info,
        Severity::Minor,
        Severity::Major,
        Severity::Blocker,
    ];
    assert_eq!(levels.len(), 4);
    // Ordering: Blocker > Major > Minor > Info (used to sort findings).
    assert!(Severity::Blocker > Severity::Major);
    assert!(Severity::Major > Severity::Minor);
    assert!(Severity::Minor > Severity::Info);
}

#[test]
fn report_synthesizes_one_finding_per_critique() {
    // Run a two-critic parallel panel over a subject; each Critique becomes a
    // finding. Prompts (the subject broadcast) must NOT become findings.
    let graph = InteractionGraph::from(Topology::parallel(vec![
        AgentId::new("critic-a"),
        AgentId::new("critic-b"),
    ]));
    let mut runner = Runner::new(graph)
        .with_agent(EchoAgent::new(
            AgentId::new("critic-a"),
            Persona::new("Critic A"),
        ))
        .with_agent(EchoAgent::new(
            AgentId::new("critic-b"),
            Persona::new("Critic B"),
        ));

    let transcript = runner
        .execute(&Subject::from_markdown(
            "# Plan\n\nDo the thing.",
            "plan.md",
        ))
        .expect("echo run never fails");
    let report = Report::from_transcript(&transcript);

    // Two critiques -> two findings; the prompt is not a finding.
    assert_eq!(report.findings().len(), 2);

    // Findings preserve critic order; each echoes the subject text (echo backend).
    let authors: Vec<&str> = report
        .findings()
        .iter()
        .map(|f| f.author().as_str())
        .collect();
    assert_eq!(authors, vec!["critic-a", "critic-b"]);

    for finding in report.findings() {
        assert_eq!(finding.summary(), "# Plan\n\nDo the thing.");
        // The echo backend cannot infer severity, so findings default to Major.
        assert_eq!(finding.severity(), &Severity::Major);
    }
}

#[test]
fn report_from_manual_transcript_skips_non_critiques() {
    // Build a transcript by hand mixing kinds: only Critiques become findings.
    let mut transcript = Transcript::new();
    transcript.push(Message::new(
        AgentId::new("system"),
        None,
        MessageKind::Prompt,
        "prompt that must not become a finding",
    ));
    transcript.push(Message::new(
        AgentId::new("critic-a"),
        None,
        MessageKind::Critique,
        "a real finding",
    ));
    transcript.push(Message::new(
        AgentId::new("critic-b"),
        None,
        MessageKind::Question,
        "a question, not a finding",
    ));

    let report = Report::from_transcript(&transcript);

    assert_eq!(report.findings().len(), 1);
    assert_eq!(report.findings()[0].author().as_str(), "critic-a");
    assert_eq!(report.findings()[0].summary(), "a real finding");
}

#[test]
fn report_renders_to_markdown_with_title_and_per_finding_blocks() {
    use praxis::Finding;

    let mut report = Report::new();
    report.push_finding(Finding::new(
        AgentId::new("critic-a"),
        Severity::Blocker,
        "Assumptions unsupported.",
    ));
    report.push_finding(Finding::new(
        AgentId::new("critic-b"),
        Severity::Info,
        "Worth a footnote.",
    ));

    let md = report.to_markdown();

    assert!(md.starts_with("# Critique Report"));
    // Each finding: numbered, with severity label and summary.
    assert!(md.contains("1. [blocker] Assumptions unsupported."));
    assert!(md.contains("2. [info] Worth a footnote."));
    // Author attribution.
    assert!(md.contains("critic-a"));
    assert!(md.contains("critic-b"));
}

#[test]
fn empty_report_renders_no_findings_message() {
    let report = Report::new();
    let md = report.to_markdown();
    assert!(md.contains("# Critique Report"));
    assert!(md.contains("No findings."));
}
