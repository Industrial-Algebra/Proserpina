// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the JSON report path.
//!
//! Requires `cli`, `backend-http`, and `json`. Only the no-network property is
//! unit-tested (the NoAuthedProviders path produces a clean error before any
//! JSON rendering); the live success path is covered by the summarizer live
//! test and the CLI smoke tests.

#![cfg(all(feature = "cli", feature = "backend-http", feature = "json"))]

use proserpina::cli::run_critique;

#[test]
fn run_critique_with_json_errors_cleanly_when_no_keys_set() {
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
    std::env::set_var("PI_HOME", "/nonexistent/pi-test");
    // Use a temp empty config file so real credentials don't leak in.
    let temp_config = tempfile::NamedTempFile::new().expect("tempfile");
    std::fs::write(temp_config.path(), "# empty\n").expect("write");
    std::env::set_var("PROSERPINA_CONFIG", temp_config.path());
    let result = run_critique(
        "# Plan\n\nbody",
        "plan.md",
        0,
        None,
        true,
        None,
        proserpina::backend::http::RetryPolicy::NONE,
        None,
    );
    assert!(result.is_err());
    std::env::remove_var("PI_HOME");
}
