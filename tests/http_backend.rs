// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the OpenAI-compatible HTTP backend.
//!
//! The HTTP backend's network call cannot be unit-tested without a live
//! server, so these tests target the backend's *pure* logic — prompt
//! rendering, request building, and response parsing — which together fully
//! determine `respond`'s behavior modulo the network round-trip.

#![cfg(feature = "backend-http")]

use proserpina::backend::http::{parse_completion_response, render_prompt, HttpAgent, HttpConfig};
use proserpina::{AgentId, Message, MessageKind, Persona};

#[test]
fn render_prompt_emits_system_message_from_persona() {
    let persona = Persona::new("Methodologist")
        .with_framing("Scrutinize the rigor of every claim.")
        .with_focus("proof gaps");
    let incoming = Message::new(
        AgentId::new("system"),
        None,
        MessageKind::Prompt,
        "Critique this roadmap.",
    );

    let prompt = render_prompt(&persona, &incoming);

    // The first message is always the system message, carrying the persona.
    assert_eq!(prompt[0].role, "system");
    assert!(prompt[0].content.contains("Methodologist"));
    assert!(prompt[0]
        .content
        .contains("Scrutinize the rigor of every claim."));
    assert!(prompt[0].content.contains("proof gaps"));
}

#[test]
fn render_prompt_carries_incoming_message_kind_and_text_as_user_turn() {
    let persona = Persona::new("Red Team");
    let incoming = Message::new(
        AgentId::new("critic-b"),
        Some(AgentId::new("red-team")),
        MessageKind::Critique,
        "Your assumption is unsupported.",
    );

    let prompt = render_prompt(&persona, &incoming);

    // A user turn carries the incoming message, including its kind so the
    // model knows whether it is being prompted or rebutted.
    let user = &prompt[1];
    assert_eq!(user.role, "user");
    assert!(user.content.contains("Your assumption is unsupported."));
    assert!(user.content.contains("critique"), "kind should be rendered");
    // The sender is named so the model knows who it is responding to.
    assert!(user.content.contains("critic-b"));
}

#[test]
fn render_prompt_instructs_the_model_to_respond_with_a_message_kind() {
    // The prompt must tell the model how to declare its reply's kind, so the
    // backend can map the response back to a MessageKind. For a Prompt the
    // expected reply kind is Critique; for a Critique it is Rebuttal.
    let persona = Persona::new("Devil's Advocate");
    let prompt_in = Message::new(AgentId::new("system"), None, MessageKind::Prompt, "doc");
    let rendered_for_prompt = render_prompt(&persona, &prompt_in);
    assert!(rendered_for_prompt.iter().any(|m| m
        .content
        .to_lowercase()
        .contains("respond with kind: critique")));

    let critique_in = Message::new(AgentId::new("c"), None, MessageKind::Critique, "claim");
    let rendered_for_critique = render_prompt(&persona, &critique_in);
    assert!(rendered_for_critique.iter().any(|m| m
        .content
        .to_lowercase()
        .contains("respond with kind: rebuttal")));
}

#[test]
fn parse_completion_response_extracts_assistant_text_as_message_body() {
    // A minimal OpenAI-compatible chat-completions response. The backend
    // pulls choices[0].message.content as the reply text.
    let body = serde_json::json!({
        "choices": [
            { "message": { "role": "assistant", "content": "Section 2 is unsupported." } }
        ]
    })
    .to_string();

    let parsed =
        parse_completion_response(&body, AgentId::new("methodologist"), MessageKind::Critique)
            .expect("well-formed body parses");

    assert_eq!(parsed.sender(), &AgentId::new("methodologist"));
    assert!(matches!(parsed.kind(), MessageKind::Critique));
    assert_eq!(parsed.text(), "Section 2 is unsupported.");
}

#[test]
fn parse_completion_response_errors_on_missing_choices() {
    let body = serde_json::json!({ "error": "rate limited" }).to_string();
    let result = parse_completion_response(&body, AgentId::new("m"), MessageKind::Critique);
    assert!(result.is_err());
}

#[test]
fn http_agent_implements_the_agent_trait() {
    use proserpina::Agent;
    let agent = HttpAgent::new(
        AgentId::new("methodologist"),
        Persona::new("Methodologist"),
        HttpConfig {
            base_url: "https://api.deepseek.com/v1".to_owned(),
            model: "deepseek-chat".to_owned(),
            api_key: "dummy-key-not-used-in-this-test".to_owned(),
        },
    );
    assert_eq!(agent.id().as_str(), "methodologist");
    assert_eq!(agent.persona().name(), "Methodologist");
}

/// End-to-end wiring check against a live DeepSeek API. Ignored by default —
/// run on demand with:
///   DEEPSEEK_API_KEY=... cargo test --features backend-http --test http_backend -- --ignored live_deepseek
///
/// Verifies the block_on bridge, request building, auth header, URL, and
/// response parsing all work against a real provider. Not run in CI (network,
/// key, cost).
#[test]
#[ignore]
fn live_deepseek_responds_with_a_critique_for_a_prompt() {
    use proserpina::Agent;
    let Ok(key) = std::env::var("DEEPSEEK_API_KEY") else {
        eprintln!("skipped: DEEPSEEK_API_KEY not set");
        return;
    };
    let mut agent = HttpAgent::new(
        AgentId::new("methodologist"),
        Persona::new("Methodologist").with_focus("unsupported assumptions"),
        HttpConfig {
            base_url: "https://api.deepseek.com/v1".to_owned(),
            model: "deepseek-chat".to_owned(),
            api_key: key,
        },
    );
    let prompt = Message::new(
        AgentId::new("system"),
        Some(AgentId::new("methodologist")),
        MessageKind::Prompt,
        "# Plan\n\nWe will prove P=NP by next quarter.",
    );

    let reply = agent.respond(&prompt).expect("live call should succeed");
    assert!(matches!(reply.kind(), MessageKind::Critique));
    assert!(
        !reply.text().trim().is_empty(),
        "reply should have body text"
    );
}
