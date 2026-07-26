// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! # Proserpina
//!
//! Multi-agent critique and cross-examination pipeline for documents that
//! require intellectual rigor — pre-prints, roadmaps, plans, and specs.
//!
//! Proserpina runs a configurable ensemble of critic *personas* over a document via
//! a **provider-agnostic interaction-graph engine**. LLM backends are pluggable
//! behind an `Agent` trait; an `EchoAgent` backend makes the entire engine
//! deterministic and testable with zero LLM dependencies.
//!
//! ## Architecture
//!
//! - A `Subject` is the document under critique.
//! - A `Topology` describes how critic agents exchange messages: `parallel`
//!   (fan-out), `rounds` (adversarial cross-examination), or `moderated`
//!   (Socratic dialectic). All three are instances of a general
//!   `InteractionGraph` — topologies are templates, not special cases.
//! - A `Runner` executes the graph against a backend agent, producing a
//!   `Transcript` and a synthesized `Report` of `Finding`s.
//!
//! > **Status:** scaffold. Modules land incrementally via test-driven
//! > development (see `docs/plans/2026-06-19-proserpina-design.md`). This file
//! > ships the crate-level documentation and the feature surface; individual
//! > modules are added in subsequent PRs.
//!
//! ## Features
//!
//! - `std` (default): standard library support
//! - `cli`: the `proserpina` binary and clap command line interface
//! - `serde`: `Serialize`/`Deserialize` impls for core types
//! - `json`: machine-readable JSON report output (implies `serde`)
//! - `backend-http`: OpenAI-compatible HTTP agent, multi-provider roster,
//!   credentials config, summarizer (implies `serde`)
//! - `keyring`: OS keychain credential tier (implies `backend-http`); macOS
//!   Keychain + Windows Credential Manager supported, Linux gnome-keyring has
//!   a known limitation
//!
//! ## Usage
//!
//! Once built out, the CLI runs the configured critic personas over a document
//! and renders a markdown critique report:
//!
//! ```text
//! proserpina critique path/to/roadmap.md -o critique.md
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

pub use proserpina_agent::{agent, error, message};

/// Critic personas — the core [`Persona`](proserpina_agent::Persona) type is
/// re-exported from `proserpina-agent`; the built-in panel presets and
/// config-aware panel resolution are critique-pipeline concerns and live
/// here.
pub mod persona {
    pub use proserpina_agent::persona::*;
    pub use crate::panels::Panel;
    #[cfg(feature = "backend-http")]
    pub use crate::panels::resolve_panel;
}

mod panels;

#[cfg(feature = "backend-http")]
pub mod agent_info;
pub mod backend;
mod graph;
mod report;
mod runner;
mod subject;
mod transcript;

#[cfg(feature = "backend-http")]
pub mod summary;

#[cfg(all(feature = "cli", feature = "backend-http"))]
pub mod auth;

pub use proserpina_agent::{Agent, AgentId, Message, MessageKind, Persona, ProserpinaError};

#[cfg(feature = "backend-http")]
pub use agent_info::{Capabilities, Plan, PlanSlot, ProviderInfo};
pub use backend::EchoAgent;
pub use graph::{InteractionGraph, Topology};
pub use report::{Finding, Report, Severity};
pub use runner::Runner;
pub use subject::Subject;
pub use transcript::Transcript;

#[cfg(feature = "cli")]
pub mod cli;
