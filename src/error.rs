// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Error types for Praxis.
//!
//! Every fallible public operation returns [`Result<_, PraxisError>`]. Library
//! code never panics; all failure modes flow through this enum.

use thiserror::Error;

use crate::agent::AgentId;

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

    /// A graph node referenced an [`AgentId`](crate::agent::AgentId) for which
    /// no agent was registered with the
    /// [`Runner`](crate::runner::Runner).
    #[error("no agent registered for id `{0}`")]
    MissingAgent(AgentId),

    /// No provider in a roster had its API key set in the environment.
    ///
    /// Carries the names of the providers that were tried.
    #[error("no API keys found for any provider (tried: {})", .0.join(", "))]
    NoAuthedProviders(Vec<String>),

    /// The summarizer LLM call failed (network, HTTP, or parse-to-empty).
    #[error("summarizer failed: {detail}")]
    SummaryFailed { detail: String },

    /// The credentials config file exists but could not be read or parsed.
    #[error("malformed credentials config `{path}`: {detail}")]
    MalformedCredentials {
        /// The config file path (or `<str>` for inline parsing).
        path: String,
        /// The underlying read/parse error.
        detail: String,
    },

    /// A custom provider (not in the registry) was declared in the config but
    /// is missing a required field.
    #[error("custom provider `{name}` is missing required field(s): {}", .missing.join(", "))]
    IncompleteCustomProvider {
        /// The custom provider's name.
        name: String,
        /// The missing fields (subset of `api_key`, `model`, `base_url`).
        missing: Vec<&'static str>,
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

    /// Convenience constructor for [`PraxisError::MissingAgent`].
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::{AgentId, PraxisError};
    /// let err = PraxisError::missing_agent(AgentId::new("ghost"));
    /// assert!(format!("{err}").contains("ghost"));
    /// ```
    pub fn missing_agent(id: AgentId) -> Self {
        Self::MissingAgent(id)
    }

    /// Convenience constructor for [`PraxisError::NoAuthedProviders`].
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::PraxisError;
    /// let err = PraxisError::no_authed_providers(vec!["deepseek".to_owned()]);
    /// assert!(format!("{err}").contains("deepseek"));
    /// ```
    pub fn no_authed_providers(tried: Vec<String>) -> Self {
        Self::NoAuthedProviders(tried)
    }

    /// Convenience constructor for [`PraxisError::SummaryFailed`].
    pub fn summary_failed(detail: impl Into<String>) -> Self {
        Self::SummaryFailed {
            detail: detail.into(),
        }
    }

    /// Convenience constructor for [`PraxisError::MalformedCredentials`].
    ///
    /// Accepts any error-like `detail` (the read/parse error's display).
    pub fn malformed_credentials(path: impl Into<String>, detail: impl std::fmt::Display) -> Self {
        Self::MalformedCredentials {
            path: path.into(),
            detail: detail.to_string(),
        }
    }

    /// Convenience constructor for [`PraxisError::IncompleteCustomProvider`].
    pub fn incomplete_custom_provider(name: impl Into<String>, missing: Vec<&'static str>) -> Self {
        Self::IncompleteCustomProvider {
            name: name.into(),
            missing,
        }
    }
}
