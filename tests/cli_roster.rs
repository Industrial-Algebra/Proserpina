// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the roster-based CLI entry point.
//!
//! Requires both `cli` and `backend-http`. Only the no-network property is
//! unit-tested here (the `NoAuthedProviders` path); the live success path is
//! covered by `examples/deepseek_smoke.rs` and the multi-provider smoke test.

#![cfg(all(feature = "cli", feature = "backend-http"))]

use proserpina::cli::run_critique;

#[test]
fn run_critique_errors_when_no_provider_keys_are_set() {
    // With the default registry and no keys in the environment, the roster
    // path must surface NoAuthedProviders rather than silently falling back
    // to echo or attempting network calls.
    // NOTE: this test assumes the standard provider keys are not set in the
    // test environment. We remove a few common ones defensively.
    for var in [
        "DEEPSEEK_API_KEY",
        "OPENAI_API_KEY",
        "MOONSHOT_API_KEY",
        "DASHSCOPE_API_KEY",
        "ZAI_API_KEY",
        "GOOGLE_API_KEY",
    ] {
        std::env::remove_var(var);
    }

    let result = run_critique(
        "# Plan\n\nbody",
        "plan.md",
        0,
        None,
        false,
        None,
        proserpina::backend::http::RetryPolicy::NONE,
    );
    let err = result.expect_err("no keys set -> should error");
    let msg = format!("{err}");
    assert!(
        msg.to_lowercase().contains("no api keys"),
        "expected a no-keys message, got: {msg}"
    );
}
