// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! # Praxis
//!
//! Multi-agent critique and cross-examination pipeline for documents that
//! require intellectual rigor — pre-prints, roadmaps, plans, and specs.
//!
//! Praxis runs a configurable ensemble of critic *personas* over a document via
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
//! > development (see `docs/plans/2026-06-19-praxis-design.md`). This file
//! > ships the crate-level documentation and the feature surface; individual
//! > modules are added in subsequent PRs.
//!
//! ## Features
//!
//! - `std` (default): standard library support
//! - `cli`: the `praxis` binary and clap command line interface
//! - `serde`: `Serialize`/`Deserialize` impls for core types
//! - `json`: machine-readable JSON report output (implies `serde`)
//!
//! ## Usage
//!
//! Once built out, the CLI runs the configured critic personas over a document
//! and renders a markdown critique report:
//!
//! ```text
//! praxis critique path/to/roadmap.md -o critique.md
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

mod agent;
mod backend;
mod error;
mod message;
mod persona;
mod subject;

pub use agent::{Agent, AgentId};
pub use backend::EchoAgent;
pub use error::PraxisError;
pub use message::{Message, MessageKind};
pub use persona::Persona;
pub use subject::Subject;
