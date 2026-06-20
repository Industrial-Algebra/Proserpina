// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The deterministic echo backend.
//!
//! [`EchoAgent`] mirrors incoming [`crate::message::Message`]s — with one
//! adjustment: a [`MessageKind::Prompt`] (the subject broadcast) elicits a
//! [`MessageKind::Critique`], because a critic's job is to critique, not to
//! echo the prompt. Every other kind is mirrored unchanged: re-stamped as
//! authored by the agent, addressed back to the original sender, with the
//! original kind and text. Output is fully determined by `(persona, input)`,
//! with no external state, which makes this the reference backend for testing
//! the engine end-to-end without any LLM dependency.

use crate::agent::{Agent, AgentId};
use crate::message::{Message, MessageKind};
use crate::persona::Persona;

/// A backend that deterministically echoes incoming messages.
///
/// Construct with [`EchoAgent::new`] from an [`AgentId`] and a [`Persona`].
/// Each call to [`Agent::respond`] returns a message authored by this agent
/// that either critiques (for a [`MessageKind::Prompt`]) or mirrors (any other
/// kind) the input's kind and text.
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
        // the original sender. A critic produces a Critique in response to a
        // Prompt; any other kind is mirrored unchanged. Fully determined by
        // (self, msg) — no hidden state.
        let kind = match msg.kind() {
            MessageKind::Prompt => MessageKind::Critique,
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
