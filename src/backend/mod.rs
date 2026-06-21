// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Backends: concrete implementations of [`crate::agent::Agent`].
//!
//! - [`echo::EchoAgent`] is the deterministic, dependency-free reference
//!   backend used to drive the engine in tests.
//! - The HTTP backend ([`http`] module, behind the `backend-http` feature)
//!   calls an OpenAI-compatible chat-completions endpoint and is the first
//!   real LLM backend.

pub mod echo;

#[cfg(feature = "backend-http")]
pub mod http;

pub use echo::EchoAgent;
