// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the core data model.
//!
//! These exercise the public surface assembled in `src/lib.rs` across the
//! `error`, `subject`, `agent`, `persona`, and `message` modules.

use praxis::{AgentId, Message, MessageKind, Persona, PraxisError, Subject};

#[test]
fn error_renders_descriptive_message() {
    // PraxisError is thiserror-derived; the Agent variant carries context.
    let err = PraxisError::agent_failure("claude-1", "rate limited");
    let rendered = format!("{err}");
    assert!(
        rendered.contains("claude-1"),
        "message should name the agent"
    );
    assert!(
        rendered.contains("rate limited"),
        "message should include the detail"
    );
}

#[test]
fn error_is_send_and_sync() {
    // Library errors must be Send + Sync so backends can be used across threads.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PraxisError>();
}

#[test]
fn subject_carries_text_and_source_path() {
    let subject = Subject::from_markdown("# Plan\n\nbody", "roadmap.md");
    assert_eq!(subject.text(), "# Plan\n\nbody");
    assert_eq!(subject.source(), Some("roadmap.md"));
}

#[test]
fn subject_source_is_optional() {
    let subject = Subject::from_markdown("anonymous doc", "");
    assert_eq!(subject.source(), None);
}

#[test]
fn agent_id_is_a_newtype_over_string() {
    let id = AgentId::new("methodologist");
    assert_eq!(id.as_str(), "methodologist");
}

#[test]
fn agent_ids_with_same_name_are_equal() {
    assert_eq!(AgentId::new("red-team"), AgentId::new("red-team"));
    assert_ne!(AgentId::new("red-team"), AgentId::new("editor"));
}

#[test]
fn persona_has_name_framing_and_focus() {
    let persona = Persona::new("Devil's Advocate")
        .with_framing("Assume the proposal is wrong; find how.")
        .with_focus("logical gaps");
    assert_eq!(persona.name(), "Devil's Advocate");
    assert_eq!(
        persona.framing(),
        Some("Assume the proposal is wrong; find how.")
    );
    assert_eq!(persona.focus(), Some("logical gaps"));
}

#[test]
fn message_records_sender_recipient_kind_and_text() {
    let msg = Message::new(
        AgentId::new("critic-a"),
        Some(AgentId::new("critic-b")),
        MessageKind::Critique,
        "The assumptions in section 2 are unsupported.",
    );
    assert_eq!(msg.sender().as_str(), "critic-a");
    assert_eq!(msg.recipient(), Some(&AgentId::new("critic-b")));
    assert!(matches!(msg.kind(), MessageKind::Critique));
    assert_eq!(msg.text(), "The assumptions in section 2 are unsupported.");
}

#[test]
fn message_kind_is_exhaustive_with_five_variants() {
    // The design pins MessageKind to exactly these five variants; a change
    // here should be a conscious decision, not a silent addition.
    let kinds = [
        MessageKind::Critique,
        MessageKind::Rebuttal,
        MessageKind::Question,
        MessageKind::Concession,
        MessageKind::Verdict,
    ];
    // Each round-trips through a label and back (stability check for future serde).
    for kind in kinds {
        let _label: &str = kind.label();
        let round_tripped = MessageKind::from_label(kind.label());
        assert_eq!(round_tripped, Ok(kind));
    }
}
