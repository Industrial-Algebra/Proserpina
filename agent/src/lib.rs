// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Provider-agnostic LLM agent abstraction.
//!
//! This crate is the reusable core of the Proserpina critique pipeline:
//! the [`Agent`] trait, [`Persona`], [`Message`] types, a deterministic
//! [`EchoAgent`] test oracle, and (behind `backend-http`) an
//! OpenAI-compatible [`HttpAgent`]. Behind `credentials`, the pure
//! credential-resolution core for daemon consumers.
//!
//! Extracted from `proserpina` v0.3.0 so daemon consumers (e.g. Ijima's
//! mining tier) can depend on the agent layer without the critique
//! pipeline, CLI, or auth subsystem.

// Modules land here incrementally as the extraction proceeds
// (see docs/plans/2026-07-26-proserpina-agent-extraction.md).
