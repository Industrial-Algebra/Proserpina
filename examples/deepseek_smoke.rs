// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Manual smoke test for the HTTP backend against a live OpenAI-compatible API.
//!
//! Run with:
//!   DEEPSEEK_API_KEY=... cargo run --features backend-http --example deepseek_smoke
//!
//! Not a unit test: it makes a real, billable network call. It exists to
//! verify the HTTP backend's wiring (block_on bridge, request building, auth,
//! response parsing) end-to-end against a real provider.

#![cfg(feature = "backend-http")]

use praxis::{
    backend::http::{HttpAgent, HttpConfig},
    Agent, AgentId, Message, MessageKind, Persona,
};

fn main() {
    let key = std::env::var("DEEPSEEK_API_KEY")
        .expect("set DEEPSEEK_API_KEY to run the deepseek smoke test");

    let mut agent = HttpAgent::new(
        AgentId::new("methodologist"),
        Persona::new("Methodologist")
            .with_framing("Scrutinize the rigor of every claim.")
            .with_focus("proof gaps and unsupported assumptions"),
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
        "# Roadmap\n\nWe will prove P=NP by next quarter.",
    );

    match agent.respond(&prompt) {
        Ok(reply) => {
            println!("=== reply kind: {:?} ===", reply.kind());
            println!("{}", reply.text());
        }
        Err(e) => {
            eprintln!("deepseek_smoke failed: {e}");
            std::process::exit(1);
        }
    }
}
