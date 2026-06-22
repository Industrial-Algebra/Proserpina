// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `praxis` command-line interface.
//!
//! The CLI is a thin wrapper over testable library entry points, so the
//! critique logic can be exercised without spawning a process:
//! - [`run_critique_echo`] — the offline echo path (always available under
//!   `cli`).
//! - [`run_critique`] — the multi-provider roster path (requires `cli` +
//!   `backend-http`).
//!
//! The binary (`src/main.rs`) parses arguments, reads the input file, calls
//! the appropriate entry point, and writes the report.

pub mod critique;

pub use critique::run_critique_echo;

#[cfg(feature = "backend-http")]
pub use critique::run_critique;
