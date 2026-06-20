// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The deterministic echo backend.
//!
//! [`EchoAgent`] models a deterministic adversarial critic. Each call to
//! [`Agent::respond`](crate::agent::Agent::respond) re-stamps the reply as
//! authored by the agent and addressed back to the original sender, with a kind
//! determined by the incoming message:
//! - a [`MessageKind::Prompt`] (the subject broadcast) elicits a
//!   [`MessageKind::Critique`];
//! - a [`MessageKind::Critique`] (another critic's finding) elicits a
//!   [`MessageKind::Rebuttal`] — this is what makes cross-examination
//!   meaningful in the rounds topology;
//! - any other kind is mirrored unchanged.
//!
//! Output is fully determined by `(persona, input)`, with no external state,
//! which makes this the reference backend for testing the engine end-to-end
//! without any LLM dependency.

use crate::agent::{Agent, AgentId};
use crate::message::{Message, MessageKind};
use crate::persona::Persona;

/// A backend that deterministically models an adversarial critic.
///
/// Construct with [`EchoAgent::new`] from an [`AgentId`] and a [`Persona`].
/// Each call to [`Agent::respond`](crate::agent::Agent::respond) returns a
/// message authored by this agent that critiques (for a
/// [`MessageKind::Prompt`]), rebuts (for a [`MessageKind::Critique`]), or
/// mirrors (any other kind) the input's kind and text.
#[derive(Debug, Clone)]
pub struct EchoAgent {
    id: AgentId,
    persona: Persona,
}

impl EchoAgent {
    /// Creates a new echo agent with the given identity and persona.
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::{Agent, AgentId, EchoAgent, Persona};
    /// let agent = EchoAgent::new(
    ///     AgentId::new("methodologist"),
    ///     Persona::new("Methodologist"),
    /// );
    /// assert_eq!(agent.id().as_str(), "methodologist");
    /// ```
    pub fn new(id: AgentId, persona: Persona) -> Self {
        Self { id, persona }
    }
}

impl Agent for EchoAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn persona(&self) -> &Persona {
        &self.persona
    }

    fn respond(&mut self, msg: &Message) -> Result<Message, crate::error::PraxisError> {
        // Re-stamp the reply as authored by this agent and address it back to
        // the original sender. The kind encodes the agent's role in the
        // dialogue:
        //   - a Prompt (the subject broadcast) elicits a Critique;
        //   - a Critique (another critic's finding) elicits a Rebuttal;
        //   - any other kind is mirrored unchanged.
        // Fully determined by (self, msg) — no hidden state.
        let kind = match msg.kind() {
            MessageKind::Prompt => MessageKind::Critique,
            MessageKind::Critique => MessageKind::Rebuttal,
            other => other,
        };
        Ok(Message::new(
            self.id.clone(),
            Some(msg.sender().clone()),
            kind,
            msg.text().to_owned(),
        ))
    }
}
