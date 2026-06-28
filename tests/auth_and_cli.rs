// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for auth validation + human CLI improvements.

#![cfg(feature = "backend-http")]

use proserpina::backend::http::{validate_provider, HttpConfig, RetryPolicy};

fn cfg(base_url: &str, key: &str) -> HttpConfig {
    HttpConfig {
        base_url: base_url.to_owned(),
        model: "test-model".to_owned(),
        api_key: key.to_owned(),
    }
}

#[test]
fn validate_provider_rejects_a_bad_key() {
    // Point at a real provider with a dummy key — should get 401/403.
    let config = cfg("https://api.deepseek.com/v1", "sk-definitely-not-real");
    let policy = RetryPolicy::NONE; // don't retry — fail fast
    let result = validate_provider(&config, &policy);
    assert!(result.is_err(), "a fake key should be rejected");
}

#[test]
fn validate_provider_rejects_a_bad_host() {
    let config = cfg("https://nonexistent-host-invalid.example/v1", "any-key");
    let policy = RetryPolicy::NONE;
    let result = validate_provider(&config, &policy);
    assert!(result.is_err(), "a bad host should fail");
}
