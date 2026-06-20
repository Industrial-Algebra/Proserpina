// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `praxis` command-line interface.
//!
//! The CLI is a thin wrapper over a testable library entry point,
//! [`run_critique`], so the critique logic can be exercised without spawning a
//! process. The binary (`src/main.rs`) parses arguments, reads the input file,
//! calls [`run_critique`], and writes the report.

pub mod critique;

pub use critique::run_critique;
