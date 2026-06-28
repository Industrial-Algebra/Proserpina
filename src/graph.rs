// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! The interaction graph and its topology constructors.
//!
//! A Proserpina run is modeled as a directed graph of agent-to-agent message
//! passing: nodes are agents (critics, a moderator, a synthesizer) and edges
//! are message routes. Topologies — `parallel`, `rounds`, `moderated` — are
//! templates that produce a graph.
//!
//! This module ships `parallel` (a degenerate single-round graph) and `rounds`
//! (adversarial cross-examination over successive rounds). `moderated` lands
//! later. Both shipped topologies are instances of the general
//! [`InteractionGraph`]; new ones are new constructors, not special cases.

use crate::agent::AgentId;

/// A topology template: a declarative description of how critics exchange
/// messages, before it is lowered into an [`InteractionGraph`].
///
/// Convert into a graph with `.into()` (or pass it directly to the runner).
#[derive(Debug, Clone)]
pub enum Topology {
    /// Fan-out: every critic receives the subject prompt independently and
    /// produces a critique. The simplest cross-examination topology — a
    /// degenerate single-round graph.
    Parallel {
        /// The critics that make up the panel, in declared order.
        critics: Vec<AgentId>,
    },
    /// Adversarial cross-examination over successive rounds.
    ///
    /// Round 1: the subject is broadcast to every critic as a prompt, each
    /// producing a critique. Rounds 2 and beyond: each critic receives the
    /// prior round's messages from the *other* critics and responds (a
    /// critique elicits a rebuttal, per the echo contract). The run stops
    /// early if a round produces no rebuttals (the panel has converged), and
    /// never runs more than `max_rounds` rounds.
    Rounds {
        /// The critics that make up the panel, in declared order.
        critics: Vec<AgentId>,
        /// The maximum number of rounds to run (including the initial
        /// critique round). `Rounds { max_rounds: 1 }` is equivalent to
        /// [`Topology::Parallel`].
        max_rounds: usize,
    },
}

impl Topology {
    /// Builds a parallel-fan-out topology from a panel of critics.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::{AgentId, InteractionGraph, Topology};
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

    /// Builds an adversarial rounds topology from a panel of critics and a
    /// maximum round count.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::{AgentId, InteractionGraph, Topology};
    ///
    /// let topology = Topology::rounds(vec![AgentId::new("a"), AgentId::new("b")], 3);
    /// let graph: InteractionGraph = topology.into();
    /// assert_eq!(graph.max_rounds(), Some(3));
    /// ```
    pub fn rounds(critics: Vec<AgentId>, max_rounds: usize) -> Self {
        Self::Rounds {
            critics,
            max_rounds,
        }
    }
}

impl From<Topology> for InteractionGraph {
    fn from(topology: Topology) -> Self {
        match topology {
            Topology::Parallel { critics } => InteractionGraph::Parallel { critics },
            Topology::Rounds {
                critics,
                max_rounds,
            } => InteractionGraph::Rounds {
                critics,
                max_rounds,
            },
        }
    }
}

/// A realized interaction graph: the executable form of a [`Topology`].
///
/// The graph records the panel and (for `Rounds`) the round cap; message
/// routing is realized by the [`Runner`](crate::runner::Runner) when it walks
/// the graph.
#[derive(Debug, Clone)]
pub enum InteractionGraph {
    /// See [`Topology::Parallel`].
    Parallel {
        /// The panel, in declared order.
        critics: Vec<AgentId>,
    },
    /// See [`Topology::Rounds`].
    Rounds {
        /// The panel, in declared order.
        critics: Vec<AgentId>,
        /// The maximum number of rounds.
        max_rounds: usize,
    },
}

impl InteractionGraph {
    /// The critics that make up this graph's panel, in declared order.
    pub fn critics(&self) -> &[AgentId] {
        match self {
            InteractionGraph::Parallel { critics } | InteractionGraph::Rounds { critics, .. } => {
                critics
            }
        }
    }

    /// The maximum number of rounds, for a `Rounds` graph; `None` for
    /// `Parallel` (a single round, no cap to report).
    pub fn max_rounds(&self) -> Option<usize> {
        match self {
            InteractionGraph::Parallel { .. } => None,
            InteractionGraph::Rounds { max_rounds, .. } => Some(*max_rounds),
        }
    }
}
