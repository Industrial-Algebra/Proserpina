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

    /// A stable machine-readable kind string for this error variant.
    ///
    /// Used by [`to_error_json`](Self::to_error_json) so agents can switch on
    /// the error type without parsing the human message.
    pub fn error_kind(&self) -> &'static str {
        match self {
            PraxisError::AgentFailure { .. } => "agent_failure",
            PraxisError::MissingAgent(_) => "missing_agent",
            PraxisError::NoAuthedProviders(_) => "no_authed_providers",
            PraxisError::SummaryFailed { .. } => "summary_failed",
            PraxisError::MalformedCredentials { .. } => "malformed_credentials",
            PraxisError::IncompleteCustomProvider { .. } => "incomplete_custom_provider",
        }
    }

    /// The exit code a CLI run should terminate with when this error occurs.
    ///
    /// Matches the scheme reported by [`exit_codes_map`] / `praxis capabilities`.
    pub fn exit_code(&self) -> u8 {
        match self {
            PraxisError::AgentFailure { .. } => 11,
            PraxisError::MissingAgent(_) => 15,
            PraxisError::NoAuthedProviders(_) => 10,
            PraxisError::SummaryFailed { .. } => 12,
            PraxisError::MalformedCredentials { .. } => 13,
            PraxisError::IncompleteCustomProvider { .. } => 14,
        }
    }

    /// Renders the error as structured JSON on stderr (requires the `json`
    /// feature).
    ///
    /// Shape: `{ "error": { "kind": "...", "message": "...", "details": {...} } }`.
    #[cfg(feature = "json")]
    pub fn to_error_json(&self) -> String {
        let details = self.details_json();
        let payload = serde_json::json!({
            "error": {
                "kind": self.error_kind(),
                "message": self.to_string(),
                "details": details,
            }
        });
        serde_json::to_string(&payload)
            .unwrap_or_else(|_| "{\"error\":{\"kind\":\"serialization_failed\"}}".to_owned())
    }

    /// Variant-specific structured details for [`to_error_json`](Self::to_error_json).
    #[cfg(feature = "json")]
    fn details_json(&self) -> serde_json::Value {
        match self {
            PraxisError::AgentFailure { agent_id, detail } => serde_json::json!({
                "agent_id": agent_id,
                "detail": detail,
            }),
            PraxisError::MissingAgent(id) => serde_json::json!({ "agent_id": id.to_string() }),
            PraxisError::NoAuthedProviders(tried) => serde_json::json!({
                "tried": tried,
            }),
            PraxisError::SummaryFailed { detail } => serde_json::json!({
                "detail": detail,
            }),
            PraxisError::MalformedCredentials { path, detail } => serde_json::json!({
                "path": path,
                "detail": detail,
            }),
            PraxisError::IncompleteCustomProvider { name, missing } => serde_json::json!({
                "provider": name,
                "missing": missing,
            }),
        }
    }
}

/// The canonical Praxis exit-code scheme, as a sorted map (code -> meaning).
///
/// Single source of truth: [`PraxisError::exit_code`] returns values from
/// this scheme, and `praxis capabilities` reports it so agents can learn it.
///
/// May appear unused on builds without `backend-http` (Capabilities is then
/// gated out); it remains the documented source of truth for the codes.
#[allow(dead_code)]
pub fn exit_codes_map() -> std::collections::BTreeMap<u8, &'static str> {
    [
        (0, "success"),
        (2, "usage error"),
        (10, "no authed providers"),
        (11, "agent (provider) failure"),
        (12, "summarizer failure"),
        (13, "malformed credentials"),
        (14, "incomplete custom provider"),
        (15, "missing agent"),
        (70, "other / internal"),
    ]
    .into_iter()
    .collect()
}
