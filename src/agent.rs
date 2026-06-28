// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The provider boundary: the [`Agent`] trait and [`AgentId`].

use crate::persona::Persona;
use std::fmt;

/// A stable identifier for an agent within a single Proserpina run.
///
/// Implemented as a newtype over `String` so identities are distinct from
/// arbitrary text and compared by name. (An optional amari-backed phantom-typed
/// variant may land behind a feature later; v1 keeps the core dependency-light.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(String);

impl AgentId {
    /// Creates a new agent identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::AgentId;
    /// let id = AgentId::new("methodologist");
    /// assert_eq!(id.as_str(), "methodologist");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The identifying name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The provider boundary.
///
/// Every backend — deterministic echo, CLI subprocess, HTTP API, MCP —
/// implements `Agent`. The trait is deliberately synchronous in v1; an async
/// variant may land behind a feature once real providers need concurrency.
///
/// `respond` is `&mut self` so stateful backends (e.g. a backend tracking turn
/// count or conversation history) can evolve without changing the signature.
pub trait Agent {
    /// This agent's stable identifier.
    fn id(&self) -> &AgentId;

    /// The persona this agent applies when critiquing.
    fn persona(&self) -> &Persona;

    /// Produces this agent's response to an incoming [`crate::message::Message`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ProserpinaError::AgentFailure`] when the backend
    /// cannot produce a response.
    fn respond(
        &mut self,
        msg: &crate::message::Message,
    ) -> Result<crate::message::Message, crate::error::ProserpinaError>;
}
