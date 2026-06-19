// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The deterministic echo backend.
//!
//! [`EchoAgent`] mirrors every incoming [`crate::message::Message`]: it
//! re-stamps the sender as itself, addresses the reply back to the original
//! sender, and echoes the original kind and text unchanged. Output is fully
//! determined by `(persona, input)`, with no external state, which makes it
//! the reference backend for testing the engine end-to-end without any LLM
//! dependency.

use crate::agent::{Agent, AgentId};
use crate::message::Message;
use crate::persona::Persona;

/// A backend that deterministically echoes incoming messages.
///
/// Construct with [`EchoAgent::new`] from an [`AgentId`] and a [`Persona`].
/// Every call to [`Agent::respond`] returns a message authored by this agent
/// that mirrors the input's kind and text.
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
        // Mirror the incoming message: re-stamp as authored by this agent,
        // address the reply back to the original sender, and echo the kind
        // and text unchanged. Fully determined by (self, msg) — no hidden state.
        Ok(Message::new(
            self.id.clone(),
            Some(msg.sender().clone()),
            msg.kind(),
            msg.text().to_owned(),
        ))
    }
}
