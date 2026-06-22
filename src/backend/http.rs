// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The OpenAI-compatible HTTP backend.
//!
//! [`HttpAgent`] implements [`crate::agent::Agent`] by calling an
//! OpenAI-compatible chat-completions endpoint (DeepSeek, OpenAI, OpenRouter,
//! any compatible provider). It is the first real LLM backend; the engine and
//! the echo backend stay fully functional and testable without it.
//!
//! ## Sync/async bridge
//!
//! The engine is synchronous ([`crate::agent::Agent::respond`] takes `&mut
//! self` and returns synchronously), but HTTP is inherently async. `HttpAgent`
//! holds its own Tokio runtime and calls [`tokio::runtime::Runtime::block_on`]
//! internally, so `respond` stays synchronous — zero churn to the engine,
//! runner, CLI, or the 45+ tests that depend on the sync trait. The async
//! burden is contained entirely inside this feature-gated module.
//!
//! ## Testability
//!
//! The network round-trip is the only thing that can't be unit-tested. The
//! pure logic that determines `respond`'s behavior — [`render_prompt`] (turn
//! the persona + incoming message into a chat conversation), request building,
//! and [`parse_completion_response`] (turn the API reply into a
//! [`crate::message::Message`]) — is exposed and unit-tested directly.

use crate::agent::{Agent, AgentId};
use crate::message::{Message, MessageKind};
use crate::persona::Persona;

/// A single chat message in the OpenAI-compatible conversation format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    /// `system`, `user`, or `assistant`.
    pub role: String,
    /// The message content.
    pub content: String,
}

/// The kind label expected in the model's reply, given the incoming message.
///
/// A `Prompt` should be answered with a `Critique`; a `Critique` with a
/// `Rebuttal` (matching the echo backend's adversarial contract); anything
/// else is mirrored. This keeps HTTP-driven cross-examination consistent with
/// the echo backend and the rounds topology.
fn expected_reply_kind(incoming: MessageKind) -> MessageKind {
    match incoming {
        MessageKind::Prompt => MessageKind::Critique,
        MessageKind::Critique => MessageKind::Rebuttal,
        other => other,
    }
}

/// Renders the chat conversation for a single `respond` call.
///
/// Emits a system message describing the persona (name, framing, focus),
/// followed by a user turn carrying the incoming message — its kind, sender,
/// and text — and an instruction to reply with the expected kind so the
/// backend can map the response back to a [`MessageKind`].
pub fn render_prompt(persona: &Persona, incoming: &Message) -> Vec<ChatMessage> {
    let mut system = format!(
        "You are {}, a critic on a peer-review panel.",
        persona.name()
    );
    if let Some(framing) = persona.framing() {
        system.push_str(&format!(" Framing: {framing}."));
    }
    if let Some(focus) = persona.focus() {
        system.push_str(&format!(" Focus: {focus}."));
    }

    let reply_kind = expected_reply_kind(incoming.kind());
    let user = format!(
        "From {}: [{}] {}\n\nRespond with kind: {}.",
        incoming.sender(),
        incoming.kind().label(),
        incoming.text(),
        reply_kind.label(),
    );

    vec![
        ChatMessage {
            role: "system".to_owned(),
            content: system,
        },
        ChatMessage {
            role: "user".to_owned(),
            content: user,
        },
    ]
}

/// Parses an OpenAI-compatible chat-completions response body into a
/// [`Message`] authored by `author`, with the given `kind` (the kind the
/// backend decided the reply should have — see [`expected_reply_kind`]).
///
/// # Errors
///
/// Returns [`crate::error::PraxisError::AgentFailure`] if the body is
/// malformed or has no choices.
pub fn parse_completion_response(
    body: &str,
    author: AgentId,
    kind: MessageKind,
) -> Result<Message, crate::error::PraxisError> {
    let parsed: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        crate::PraxisError::agent_failure(author.as_str(), format!("invalid JSON response: {e}"))
    })?;

    let content = parsed
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| {
            crate::PraxisError::agent_failure(
                author.as_str(),
                "response had no choices[0].message.content",
            )
        })?;

    Ok(Message::new(author, None, kind, content.to_owned()))
}

/// Configuration for an [`HttpAgent`]: where to call and how to authenticate.
///
/// Works with any OpenAI-compatible chat-completions endpoint.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// The base URL of the API (without `/chat/completions`). For DeepSeek:
    /// `https://api.deepseek.com/v1`.
    pub base_url: String,
    /// The model to request, e.g. `deepseek-chat`, `gpt-4o-mini`.
    pub model: String,
    /// The API key. Read from the environment (e.g. `DEEPSEEK_API_KEY`) at
    /// call sites, not hard-coded.
    pub api_key: String,
}

/// Builds the JSON body for a chat-completions request.
fn build_request_body(model: &str, messages: &[ChatMessage]) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": messages,
    })
}

/// An [`Agent`] backed by an OpenAI-compatible HTTP endpoint.
///
/// Construct with [`HttpAgent::new`] from an [`AgentId`], a [`Persona`], and
/// an [`HttpConfig`]. Each [`Agent::respond`] renders the persona + incoming
/// message into a chat conversation, POSTs it to the completions endpoint,
/// and parses the reply into a [`Message`] whose kind follows the adversarial
/// contract (Critique for a Prompt, Rebuttal for a Critique, else mirrored).
pub struct HttpAgent {
    id: AgentId,
    persona: Persona,
    config: HttpConfig,
    runtime: tokio::runtime::Runtime,
    client: reqwest::Client,
}

impl HttpAgent {
    /// Creates a new HTTP agent.
    ///
    /// Spawns a dedicated Tokio runtime (used to bridge the synchronous
    /// [`Agent::respond`] to async HTTP) and a reusable reqwest client.
    pub fn new(id: AgentId, persona: Persona, config: HttpConfig) -> Self {
        Self {
            id,
            persona,
            config,
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("praxis: failed to build tokio runtime for HttpAgent"),
            client: reqwest::Client::new(),
        }
    }

    /// The async inner: render, POST, return the raw response body.
    async fn fetch_response(&self, incoming: &Message) -> Result<String, crate::PraxisError> {
        let messages = render_prompt(&self.persona, incoming);
        let body = build_request_body(&self.config.model, &messages);
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                crate::PraxisError::agent_failure(self.id.as_str(), format!("HTTP send: {e}"))
            })?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| {
            crate::PraxisError::agent_failure(self.id.as_str(), format!("HTTP body: {e}"))
        })?;
        if !status.is_success() {
            return Err(crate::PraxisError::agent_failure(
                self.id.as_str(),
                format!("HTTP {status}: {text}"),
            ));
        }
        Ok(text)
    }
}

impl Agent for HttpAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn persona(&self) -> &Persona {
        &self.persona
    }

    fn respond(&mut self, incoming: &Message) -> Result<Message, crate::PraxisError> {
        // Bridge sync -> async: block on this agent's runtime.
        let body = self.runtime.block_on(self.fetch_response(incoming))?;
        let kind = expected_reply_kind(incoming.kind());
        let author = self.id.clone();
        let mut reply = parse_completion_response(&body, author, kind)?;
        // Address the reply to whoever prompted this agent, matching the echo
        // contract (None for broadcasts stays None).
        reply = Message::new(
            self.id.clone(),
            Some(incoming.sender().clone()),
            reply.kind(),
            reply.text().to_owned(),
        );
        Ok(reply)
    }
}
