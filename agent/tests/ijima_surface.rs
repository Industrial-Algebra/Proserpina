// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Compile-and-behavior proof of Ijima's exact integration surface,
//! replicated against `proserpina-agent` standalone (no proserpina dep).
//!
//! Ijima uses (ijima-miner/src/llm.rs, ijima-server/src/api.rs):
//!   Agent, AgentId, Message, MessageKind, Persona, ProserpinaError,
//!   HttpAgent, HttpConfig, `dyn Agent`, and a scripted test agent.

#![cfg(feature = "backend-http")]

use proserpina_agent::http::{HttpAgent, HttpConfig};
use proserpina_agent::{Agent, AgentId, Message, MessageKind, Persona, ProserpinaError};
use std::sync::OnceLock;

/// Mirrors ijima-miner's scripted Stub agent: implements Agent directly.
struct Stub;

impl Agent for Stub {
    fn id(&self) -> &AgentId {
        static ID: OnceLock<AgentId> = OnceLock::new();
        ID.get_or_init(|| AgentId::new("stub"))
    }

    fn persona(&self) -> &Persona {
        static P: OnceLock<Persona> = OnceLock::new();
        P.get_or_init(|| Persona::new("stub"))
    }

    fn respond(&mut self, msg: &Message) -> Result<Message, ProserpinaError> {
        Ok(Message::new(
            self.id().clone(),
            None,
            MessageKind::Critique,
            format!("canned: {}", msg.text()),
        ))
    }
}

#[test]
fn ijima_style_dyn_agent_single_shot() {
    let mut stub = Stub;
    // ijima-server passes Option<&mut dyn proserpina::Agent> into spawn_blocking.
    let agent_dyn: &mut dyn Agent = &mut stub;
    let prompt = Message::new(
        AgentId::new("ijima-miner"),
        None,
        MessageKind::Prompt,
        "extract facts from this session",
    );
    let out = agent_dyn.respond(&prompt).unwrap();
    assert!(out.text().starts_with("canned: "));
    assert_eq!(out.kind(), MessageKind::Critique);
}

#[test]
fn ijima_style_http_agent_construction() {
    // ijima-server's build_mining_agent: env-driven HttpConfig + one persona.
    let persona = Persona::new("Session Mining Extractor")
        .with_framing("You mine session transcripts for durable facts.")
        .with_focus("decisions, chosen tools, stated constraints");
    let config = HttpConfig {
        base_url: "https://api.deepseek.com/v1".to_string(),
        model: "deepseek-chat".to_string(),
        api_key: "sk-test".to_string(),
    };
    // HttpAgent::new returns Self (infallible); construction must not panic.
    let _agent = HttpAgent::new(AgentId::new("ijima-miner"), persona, config);
}
