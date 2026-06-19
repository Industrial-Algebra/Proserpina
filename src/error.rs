// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Error types for Praxis.
//!
//! Every fallible public operation returns [`Result<_, PraxisError>`]. Library
//! code never panics; all failure modes flow through this enum.

use thiserror::Error;

/// The single error type for all of Praxis.
///
/// Variants are deliberately coarse at the scaffold stage and will gain
/// structure (e.g. dedicated graph or report variants) as modules land. Each
/// variant carries enough context to locate the failure.
#[derive(Debug, Error)]
pub enum PraxisError {
    /// An [`crate::agent::Agent`] backend failed to produce a response.
    ///
    /// `agent_id` names the offending agent; `detail` is a backend-supplied
    /// explanation (network error, malformed output, rate limit, etc.).
    #[error("agent `{agent_id}` failed: {detail}")]
    AgentFailure {
        /// Identifier of the agent that failed.
        agent_id: String,
        /// Backend-supplied explanation.
        detail: String,
    },
}

impl PraxisError {
    /// Convenience constructor for [`PraxisError::AgentFailure`].
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::PraxisError;
    /// let err = PraxisError::agent_failure("claude-1", "rate limited");
    /// assert!(format!("{err}").contains("claude-1"));
    /// ```
    pub fn agent_failure(agent_id: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::AgentFailure {
            agent_id: agent_id.into(),
            detail: detail.into(),
        }
    }
}
