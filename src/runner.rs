// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The runner: executes an [`InteractionGraph`] against a registry of agents.
//!
//! A [`Runner`] owns the executable state of a run — the graph plus a registry
//! of [`Agent`](crate::agent::Agent)s keyed by [`AgentId`] — and produces a
//! [`Transcript`] when executed over a [`Subject`](crate::subject::Subject).
//!
//! `execute` takes `&mut self` because agents are stateful
//! ([`Agent::respond`](crate::agent::Agent::respond) takes `&mut self`); this
//! keeps the synchronous core free of interior mutability. The CLI constructs
//! a runner once and calls `execute` once per run, so the borrow is no burden.

use std::collections::HashMap;

use crate::agent::{Agent, AgentId};
use crate::error::PraxisError;
use crate::graph::InteractionGraph;
use crate::message::{Message, MessageKind};
use crate::subject::Subject;
use crate::transcript::Transcript;

/// The reserved sender identity for system-originated prompts.
///
/// In a parallel topology the subject is broadcast to every critic as a
/// [`MessageKind::Critique`] message authored by `system`; critics reply back
/// to `system` (per the echo contract and the synthesizer routing).
pub const SYSTEM_AGENT: &str = "system";

/// Executes an [`InteractionGraph`] against a registry of agents.
pub struct Runner {
    graph: InteractionGraph,
    agents: HashMap<AgentId, Box<dyn Agent>>,
}

impl Runner {
    /// Creates a new runner over the given graph with an empty agent registry.
    ///
    /// Register agents with [`Runner::with_agent`] before calling
    /// [`Runner::execute`].
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::{AgentId, InteractionGraph, Runner, Topology};
    /// let graph = InteractionGraph::from(Topology::parallel(vec![AgentId::new("a")]));
    /// let runner = Runner::new(graph);
    /// ```
    pub fn new(graph: InteractionGraph) -> Self {
        Self {
            graph,
            agents: HashMap::new(),
        }
    }

    /// Registers an agent under its own [`AgentId`].
    ///
    /// Calling this twice with the same id replaces the previously registered
    /// agent. Returns `self` for chaining.
    #[must_use]
    pub fn with_agent(mut self, agent: impl Agent + 'static) -> Self {
        let id = agent.id().clone();
        self.agents.insert(id, Box::new(agent));
        self
    }

    /// Executes the graph over the given subject, returning the transcript.
    ///
    /// For the v1 `Parallel` topology: the subject is broadcast to every critic
    /// (in declared order) as a [`MessageKind::Critique`] prompt from
    /// [`SYSTEM_AGENT`]; each critic's response is appended to the transcript.
    ///
    /// # Errors
    ///
    /// Returns [`PraxisError::AgentFailure`] if a registered agent fails to
    /// respond, or [`PraxisError::MissingAgent`] if a graph node has no agent
    /// registered under its id.
    pub fn execute(&mut self, subject: &Subject) -> Result<Transcript, PraxisError> {
        let mut transcript = Transcript::new();
        let system = AgentId::new(SYSTEM_AGENT);

        for critic_id in self.graph.critics() {
            let prompt = Message::new(
                system.clone(),
                Some(critic_id.clone()),
                MessageKind::Critique,
                subject.text().to_owned(),
            );

            let agent = self
                .agents
                .get_mut(critic_id)
                .ok_or_else(|| PraxisError::missing_agent(critic_id.clone()))?;

            let response = agent.respond(&prompt)?;
            transcript.push(response);
        }

        Ok(transcript)
    }
}
