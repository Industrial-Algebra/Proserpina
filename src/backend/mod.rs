// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Backends: concrete implementations of [`crate::agent::Agent`].
//!
//! The echo backend is the deterministic, dependency-free reference backend
//! used to drive the engine in tests. Real backends (CLI subprocess, HTTP API,
//! MCP) land in later feature-gated modules.

pub mod echo;

pub use echo::EchoAgent;
