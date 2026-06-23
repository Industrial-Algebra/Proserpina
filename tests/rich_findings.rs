// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for rich findings and the summarizer.

#![cfg(feature = "backend-http")]

use praxis::backend::http::HttpConfig;
use praxis::summary::{parse_findings, render_summary_prompt, summarize};
use praxis::{AgentId, Finding, Severity};
use praxis::{Message, MessageKind, Subject, Transcript};

#[test]
fn finding_carries_the_full_rich_field_set() {
    let f = Finding::new(Severity::Blocker, "Section 2 is unsupported")
        .with_category("methodology")
        .with_location("§2")
        .with_quote("We will prove P=NP.")
        .with_suggested_change("Add a proof sketch or weaken the claim.")
        .with_supporting_critics(vec![
            AgentId::new("methodologist"),
            AgentId::new("red-team"),
        ]);

    assert_eq!(f.severity(), &Severity::Blocker);
    assert_eq!(f.summary(), "Section 2 is unsupported");
    assert_eq!(f.category.as_deref(), Some("methodology"));
    assert_eq!(f.location.as_deref(), Some("§2"));
    assert_eq!(f.quote.as_deref(), Some("We will prove P=NP."));
    assert_eq!(
        f.suggested_change.as_deref(),
        Some("Add a proof sketch or weaken the claim.")
    );
    assert_eq!(
        f.supporting_critics(),
        &[AgentId::new("methodologist"), AgentId::new("red-team")]
    );
}

#[test]
fn finding_minimal_has_no_optional_fields() {
    let f = Finding::new(Severity::Minor, "typo in intro");
    assert_eq!(f.severity(), &Severity::Minor);
    assert_eq!(f.summary(), "typo in intro");
    assert!(f.category.is_none());
    assert!(f.location.is_none());
    assert!(f.quote.is_none());
    assert!(f.suggested_change.is_none());
    assert!(f.supporting_critics().is_empty());
}

#[test]
fn finding_builder_is_chainable_and_must_use() {
    // Each with_* returns Self so the builder chains; field assignment is
    // observable on the final value.
    let f = Finding::new(Severity::Info, "note")
        .with_category("style")
        .with_location("line 3");
    assert_eq!(f.category.as_deref(), Some("style"));
    assert_eq!(f.location.as_deref(), Some("line 3"));
}

// ---- praxis-finding block parser ----

#[test]
fn parse_findings_extracts_a_single_well_formed_block() {
    let body = "Some preamble from the model.\n\n```praxis-finding\nseverity: blocker\ncategory: methodology\nsummary: Section 2 is unsupported.\nlocation: §2\nquote: We will prove P=NP.\nsuggested_change: Add a proof sketch.\nsupporting_critics: methodologist, red-team\n```\n\nSome trailing prose.";

    let findings = parse_findings(body);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.severity(), &Severity::Blocker);
    assert_eq!(f.summary(), "Section 2 is unsupported.");
    assert_eq!(f.category.as_deref(), Some("methodology"));
    assert_eq!(f.location.as_deref(), Some("§2"));
    assert_eq!(f.quote.as_deref(), Some("We will prove P=NP."));
    assert_eq!(f.suggested_change.as_deref(), Some("Add a proof sketch."));
    assert_eq!(
        f.supporting_critics(),
        &[AgentId::new("methodologist"), AgentId::new("red-team")]
    );
}

#[test]
fn parse_findings_extracts_multiple_blocks() {
    let body = "```praxis-finding\nseverity: major\nsummary: First issue.\n```\n\nbetween\n\n```praxis-finding\nseverity: minor\nsummary: Second issue.\n```";
    let findings = parse_findings(body);
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].summary(), "First issue.");
    assert_eq!(findings[0].severity(), &Severity::Major);
    assert_eq!(findings[1].summary(), "Second issue.");
    assert_eq!(findings[1].severity(), &Severity::Minor);
}

#[test]
fn parse_findings_treats_unknown_severity_as_major() {
    // Graceful degradation: an unrecognized severity doesn't fail the parse;
    // it defaults to Major so the finding is still surfaced.
    let body = "```praxis-finding\nseverity: critical-ish\nsummary: X.\n```";
    let findings = parse_findings(body);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity(), &Severity::Major);
}

#[test]
fn parse_findings_accepts_optional_fields_as_none() {
    let body = "```praxis-finding\nseverity: info\nsummary: Just a note.\n```";
    let findings = parse_findings(body);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert!(f.category.is_none());
    assert!(f.location.is_none());
    assert!(f.quote.is_none());
    assert!(f.suggested_change.is_none());
    assert!(f.supporting_critics().is_empty());
}

#[test]
fn parse_findings_returns_empty_when_no_blocks_present() {
    // The model produced only prose, no fenced blocks -> no findings.
    let body = "The model rambled but emitted no structured blocks.";
    assert!(parse_findings(body).is_empty());
}

#[test]
fn parse_findings_supporting_critics_handles_single_name() {
    let body =
        "```praxis-finding\nseverity: major\nsummary: X.\nsupporting_critics: lone-critic\n```";
    let findings = parse_findings(body);
    assert_eq!(
        findings[0].supporting_critics(),
        &[AgentId::new("lone-critic")]
    );
}

// ---- render_summary_prompt ----

#[test]
fn render_summary_prompt_includes_subject_and_transcript_turns() {
    let subject = Subject::from_markdown("# Plan\n\nDo the thing.", "plan.md");
    let mut transcript = Transcript::new();
    transcript.push(Message::new(
        AgentId::new("critic-a"),
        None,
        MessageKind::Critique,
        "The plan is vague.",
    ));
    transcript.push(Message::new(
        AgentId::new("critic-b"),
        None,
        MessageKind::Critique,
        "No timeline.",
    ));

    let prompt = render_summary_prompt(&subject, &transcript);

    // System message instructs the model to emit praxis-finding blocks.
    assert_eq!(prompt[0].role, "system");
    assert!(prompt[0].content.contains("praxis-finding"));

    // User message carries the subject text and each transcript turn.
    assert_eq!(prompt[1].role, "user");
    assert!(prompt[1].content.contains("Do the thing."));
    assert!(prompt[1].content.contains("The plan is vague."));
    assert!(prompt[1].content.contains("No timeline."));
    // Each critic turn is attributed.
    assert!(prompt[1].content.contains("critic-a"));
    assert!(prompt[1].content.contains("critic-b"));
}

/// End-to-end summarizer check against a live DeepSeek API. Ignored by default —
/// run on demand with:
///   DEEPSEEK_API_KEY=... cargo test --features backend-http --test rich_findings -- --ignored live_deepseek_summary
///
/// Verifies the block_on bridge, prompt rendering, HTTP call, and response
/// parsing all work against a real provider. Not run in CI (network, key,
/// cost).
#[test]
#[ignore]
fn live_deepseek_summary_produces_structured_findings() {
    let Ok(key) = std::env::var("DEEPSEEK_API_KEY") else {
        eprintln!("skipped: DEEPSEEK_API_KEY not set");
        return;
    };
    let subject =
        Subject::from_markdown("# Plan\n\nWe will prove P=NP by next quarter.", "plan.md");
    let mut transcript = Transcript::new();
    transcript.push(Message::new(
        AgentId::new("methodologist"),
        None,
        MessageKind::Critique,
        "There is no proof strategy. The timeline is absurd. Known barriers (relativization, natural proofs) are unaddressed.",
    ));

    let config = HttpConfig {
        base_url: "https://api.deepseek.com/v1".to_owned(),
        model: "deepseek-chat".to_owned(),
        api_key: key,
    };
    let findings =
        summarize(&subject, &transcript, &config).expect("live summarizer call should succeed");
    assert!(!findings.is_empty(), "summarizer should produce findings");
    // Each finding must have a severity and summary.
    for f in &findings {
        assert!(!f.summary().is_empty());
    }
    eprintln!("live summary produced {} findings:", findings.len());
    for (i, f) in findings.iter().enumerate() {
        eprintln!(
            "  [{}] {:?} {} | loc={:?} cat={:?} change={:?} critics={:?}",
            i + 1,
            f.severity(),
            f.summary(),
            f.location,
            f.category,
            f.suggested_change,
            f.supporting_critics(),
        );
    }
}
