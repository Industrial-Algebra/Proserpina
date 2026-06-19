// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The interaction graph and its topology constructors.
//!
//! A Praxis run is modeled as a directed graph of agent-to-agent message
//! passing: nodes are agents (critics, a moderator, a synthesizer) and edges
//! are message routes. Topologies — `parallel`, `rounds`, `moderated` — are
//! templates that produce a graph.
//!
//! v1 ships only [`Topology::parallel`], which is a degenerate single-round
//! graph: every critic receives the same subject prompt and replies to the
//! synthesizer. The structure generalizes: `rounds` and `moderated` will add
//! inter-critic edges in later PRs without changing the core types.

use crate::agent::AgentId;

/// A topology template: a declarative description of how critics exchange
/// messages, before it is lowered into an [`InteractionGraph`].
///
/// Construct one with [`Topology::parallel`] (v1) and convert it into a graph
/// with `.into()` (or pass it directly to the runner).
#[derive(Debug, Clone)]
pub enum Topology {
    /// Fan-out: every critic receives the subject prompt independently and
    /// replies to the synthesizer. The simplest cross-examination topology —
    /// a degenerate single-round graph.
    Parallel {
        /// The critics that make up the panel, in declared order.
        critics: Vec<AgentId>,
    },
}

impl Topology {
    /// Builds a parallel-fan-out topology from a panel of critics.
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::{AgentId, InteractionGraph, Topology};
    ///
    /// let topology = Topology::parallel(vec![
    ///     AgentId::new("methodologist"),
    ///     AgentId::new("red-team"),
    /// ]);
    /// let graph: InteractionGraph = topology.into();
    /// assert_eq!(graph.critics().len(), 2);
    /// ```
    pub fn parallel(critics: Vec<AgentId>) -> Self {
        Self::Parallel { critics }
    }
}

impl From<Topology> for InteractionGraph {
    fn from(topology: Topology) -> Self {
        match topology {
            Topology::Parallel { critics } => Self { critics },
        }
    }
}

/// A realized interaction graph: the executable form of a [`Topology`].
///
/// For v1 (`Parallel`), the graph is the ordered critic set; message routing
/// is implicit (subject → each critic → synthesizer). Inter-critic edges land
/// alongside the `rounds` topology.
#[derive(Debug, Clone)]
pub struct InteractionGraph {
    critics: Vec<AgentId>,
}

impl InteractionGraph {
    /// The critics that make up this graph's panel, in declared order.
    pub fn critics(&self) -> &[AgentId] {
        &self.critics
    }
}
