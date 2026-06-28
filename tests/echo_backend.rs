// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the deterministic echo backend.

use proserpina::{Agent, AgentId, EchoAgent, Message, MessageKind, Persona};

#[test]
fn echo_agent_exposes_its_id_and_persona() {
    let id = AgentId::new("methodologist");
    let persona = Persona::new("Methodologist").with_focus("rigor of proofs");
    let agent = EchoAgent::new(id, persona);

    assert_eq!(agent.id().as_str(), "methodologist");
    assert_eq!(agent.persona().name(), "Methodologist");
    assert_eq!(agent.persona().focus(), Some("rigor of proofs"));
}

#[test]
fn echo_agent_produces_a_critique_when_prompted() {
    let mut agent = EchoAgent::new(AgentId::new("critic-a"), Persona::new("Critic A"));
    let prompt = Message::new(
        AgentId::new("system"),
        Some(AgentId::new("critic-a")),
        MessageKind::Prompt,
        "Critique the roadmap.",
    );

    let reply = agent.respond(&prompt).expect("echo never fails");

    // Authored by the agent itself.
    assert_eq!(reply.sender(), &AgentId::new("critic-a"));
    // Addressed back to whoever prompted it.
    assert_eq!(reply.recipient(), Some(&AgentId::new("system")));
    // A critic produces a Critique in response to a Prompt.
    assert!(matches!(reply.kind(), MessageKind::Critique));
    // The subject text is echoed into the critique.
    assert_eq!(reply.text(), "Critique the roadmap.");
}

#[test]
fn echo_agent_rebuts_an_incoming_critique() {
    // Cross-examination: when a critic hears another critic's critique, the
    // echo backend rebuts it (rather than mirroring it as another critique).
    // This is what makes the rounds topology semantically meaningful.
    let mut agent = EchoAgent::new(AgentId::new("critic-a"), Persona::new("Critic A"));
    let critique = Message::new(
        AgentId::new("critic-b"),
        Some(AgentId::new("critic-a")),
        MessageKind::Critique,
        "The proof is unsound.",
    );

    let reply = agent.respond(&critique).expect("echo never fails");

    assert_eq!(reply.sender(), &AgentId::new("critic-a"));
    assert_eq!(reply.recipient(), Some(&AgentId::new("critic-b")));
    assert!(matches!(reply.kind(), MessageKind::Rebuttal));
    assert_eq!(reply.text(), "The proof is unsound.");
}

#[test]
fn echo_agent_mirrors_non_critique_kinds_unchanged() {
    // For any kind other than Prompt, the echo backend mirrors the kind.
    let mut agent = EchoAgent::new(AgentId::new("critic-a"), Persona::new("Critic A"));
    let incoming = Message::new(
        AgentId::new("critic-b"),
        Some(AgentId::new("critic-a")),
        MessageKind::Rebuttal,
        "Your claim is unsupported.",
    );

    let reply = agent.respond(&incoming).expect("echo never fails");

    assert!(matches!(reply.kind(), MessageKind::Rebuttal));
    assert_eq!(reply.text(), "Your claim is unsupported.");
}

#[test]
fn echo_agent_is_deterministic() {
    // Determinism is the reason EchoAgent exists — it must not depend on
    // hidden state, time, or call order. Same input twice -> identical output.
    let mut agent = EchoAgent::new(AgentId::new("critic-a"), Persona::new("Critic A"));
    let incoming = Message::new(
        AgentId::new("system"),
        None,
        MessageKind::Question,
        "Is the proof sound?",
    );

    let first = agent.respond(&incoming).expect("echo never fails");
    let second = agent.respond(&incoming).expect("echo never fails");

    assert_eq!(first, second);
}

#[test]
fn echo_agent_replies_to_sender_even_for_broadcasts() {
    // The graph routes replies back to whoever prompted the agent, even when
    // the incoming message was a broadcast (recipient == None).
    let mut agent = EchoAgent::new(AgentId::new("critic-b"), Persona::new("Critic B"));
    let broadcast = Message::new(
        AgentId::new("moderator"),
        None,
        MessageKind::Critique,
        "Open the floor.",
    );

    let reply = agent.respond(&broadcast).expect("echo never fails");

    assert_eq!(reply.sender(), &AgentId::new("critic-b"));
    assert_eq!(reply.recipient(), Some(&AgentId::new("moderator")));
}
