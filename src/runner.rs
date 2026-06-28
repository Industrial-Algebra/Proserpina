// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

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
use crate::error::ProserpinaError;
use crate::graph::InteractionGraph;
use crate::message::{Message, MessageKind};
use crate::subject::Subject;
use crate::transcript::Transcript;

/// The reserved sender identity for system-originated prompts.
///
/// The subject is broadcast to every critic as a [`MessageKind::Prompt`]
/// message authored by `system`; critics reply back to `system` (per the echo
/// contract and the synthesizer routing).
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
    /// use proserpina::{AgentId, InteractionGraph, Runner, Topology};
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
    /// Dispatches on the graph's topology:
    /// - [`InteractionGraph::Parallel`] — broadcasts the subject to each critic
    ///   and collects one critique each.
    /// - [`InteractionGraph::Rounds`] — round 1 as above; subsequent rounds
    ///   route each critic the prior round's messages from the *other* critics
    ///   (a critique elicits a rebuttal), stopping early once a round produces
    ///   no rebuttals and never exceeding `max_rounds`.
    ///
    /// # Errors
    ///
    /// Returns [`ProserpinaError::AgentFailure`] if a registered agent fails to
    /// respond, or [`ProserpinaError::MissingAgent`] if a graph node has no agent
    /// registered under its id.
    pub fn execute(&mut self, subject: &Subject) -> Result<Transcript, ProserpinaError> {
        let system = AgentId::new(SYSTEM_AGENT);
        match self.graph.clone() {
            InteractionGraph::Parallel { .. } => self.execute_parallel(subject, &system),
            InteractionGraph::Rounds { max_rounds, .. } => {
                self.execute_rounds(subject, &system, max_rounds)
            }
        }
    }

    /// Returns a mutable reference to the agent registered under `id`.
    fn agent_mut(&mut self, id: &AgentId) -> Result<&mut Box<dyn Agent>, ProserpinaError> {
        self.agents
            .get_mut(id)
            .ok_or_else(|| ProserpinaError::missing_agent(id.clone()))
    }

    fn execute_parallel(
        &mut self,
        subject: &Subject,
        system: &AgentId,
    ) -> Result<Transcript, ProserpinaError> {
        let mut transcript = Transcript::new();
        let critics: Vec<AgentId> = self.graph.critics().to_vec();

        for critic_id in &critics {
            let prompt = Message::new(
                system.clone(),
                Some(critic_id.clone()),
                MessageKind::Prompt,
                subject.text().to_owned(),
            );
            let response = self.agent_mut(critic_id)?.respond(&prompt)?;
            transcript.push(response);
        }
        Ok(transcript)
    }

    fn execute_rounds(
        &mut self,
        subject: &Subject,
        system: &AgentId,
        max_rounds: usize,
    ) -> Result<Transcript, ProserpinaError> {
        let mut transcript = Transcript::new();
        if max_rounds == 0 {
            return Ok(transcript);
        }
        let critics: Vec<AgentId> = self.graph.critics().to_vec();

        // Round 1: subject prompt -> critique, per critic.
        for critic_id in &critics {
            let prompt = Message::new(
                system.clone(),
                Some(critic_id.clone()),
                MessageKind::Prompt,
                subject.text().to_owned(),
            );
            let response = self.agent_mut(critic_id)?.respond(&prompt)?;
            transcript.push(response);
        }

        // Rounds 2..=max_rounds: each critic receives the prior round's
        // messages from the OTHER critics and responds. Stop early once a
        // round produces no rebuttals (the panel has converged).
        let mut prev_round_start = 0usize;
        for _round in 2..=max_rounds {
            // Snapshot the prior round so we can borrow `agents` mutably while
            // iterating it (cloning is cheap; messages are small).
            let prior_round: Vec<Message> =
                transcript.iter().skip(prev_round_start).cloned().collect();
            let current_round_start = transcript.len();
            let mut rebuttals_this_round = 0usize;

            for critic_id in &critics {
                for prior_msg in &prior_round {
                    if prior_msg.sender() == critic_id {
                        continue; // a critic does not rebut itself
                    }
                    let response = self.agent_mut(critic_id)?.respond(prior_msg)?;
                    if matches!(response.kind(), MessageKind::Rebuttal) {
                        rebuttals_this_round += 1;
                    }
                    transcript.push(response);
                }
            }

            if rebuttals_this_round == 0 {
                break; // converged: nothing left to challenge
            }
            prev_round_start = current_round_start;
        }

        Ok(transcript)
    }
}
