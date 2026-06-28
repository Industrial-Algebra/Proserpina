// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the keyring credential tier.
//!
//! The OS keychain can't be safely mutated in unit tests, so these tests drive
//! the pure resolution core with an explicit keyring-key snapshot (same pattern
//! as the env-key snapshot in resolve_configs). The real-keychain read is a
//! thin wrapper covered by an #[ignore] live test.

#![cfg(all(feature = "backend-http", feature = "keyring"))]

use std::collections::HashMap;

use proserpina::backend::credentials::{resolve_configs_with_keyring, Credentials};
use proserpina::backend::roster::Provider;

fn env(keys: &[(&str, &str)]) -> HashMap<String, String> {
    keys.iter()
        .copied()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

fn registry() -> Vec<Provider> {
    vec![
        Provider::new("deepseek")
            .with_base_url("https://api.deepseek.com/v1")
            .with_model("deepseek-chat")
            .with_key_env_var("DEEPSEEK_API_KEY"),
        Provider::new("zai")
            .with_base_url("https://api.z.ai/api/coding/paas/v4")
            .with_model("glm-5.2")
            .with_key_env_var("ZAI_API_KEY"),
    ]
}

#[test]
fn keyring_key_wins_over_env_and_config() {
    // A keychain entry (keyed by env-var name) takes precedence over both an
    // env var of the same name and a config-file api_key.
    let keyring = env(&[("DEEPSEEK_API_KEY", "sk-keychain")]);
    let env_keys = env(&[("DEEPSEEK_API_KEY", "sk-env")]);
    let creds = Credentials::from_toml(
        r#"[deepseek]
api_key = "sk-config""#,
    )
    .unwrap();

    let configs =
        resolve_configs_with_keyring(&registry(), &creds, &env_keys, &keyring).expect("ok");
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].api_key, "sk-keychain", "keyring must win");
}

#[test]
fn env_used_when_keyring_entry_absent() {
    // No keychain entry for deepseek; env var present -> env used.
    let keyring = HashMap::new();
    let env_keys = env(&[("DEEPSEEK_API_KEY", "sk-env")]);
    let creds = Credentials::default();

    let configs =
        resolve_configs_with_keyring(&registry(), &creds, &env_keys, &keyring).expect("ok");
    assert_eq!(configs[0].api_key, "sk-env");
}

#[test]
fn keyring_entry_under_wrong_name_is_ignored() {
    // The keyring tier is keyed by the provider's env-var name. An entry under
    // a different name does NOT satisfy a different provider.
    let keyring = env(&[("WRONG_NAME", "sk-x")]);
    let env_keys = HashMap::new();
    let creds = Credentials::default();

    let configs =
        resolve_configs_with_keyring(&registry(), &creds, &env_keys, &keyring).expect("ok");
    assert!(
        configs.is_empty(),
        "no matching keyring/env/config -> not authed"
    );
}

#[test]
fn keyring_and_env_can_auth_different_providers_simultaneously() {
    // deepseek via keychain, zai via env — both resolve.
    let keyring = env(&[("DEEPSEEK_API_KEY", "sk-keychain")]);
    let env_keys = env(&[("ZAI_API_KEY", "sk-env")]);
    let creds = Credentials::default();

    let configs =
        resolve_configs_with_keyring(&registry(), &creds, &env_keys, &keyring).expect("ok");
    assert_eq!(configs.len(), 2);
    let by_model: HashMap<&str, &str> = configs
        .iter()
        .map(|c| (c.model.as_str(), c.api_key.as_str()))
        .collect();
    assert_eq!(by_model["deepseek-chat"], "sk-keychain");
    assert_eq!(by_model["glm-5.2"], "sk-env");
}

/// Live keychain round-trip: writes a `proserpina:DEEPSEEK_API_KEY` entry, reads
/// it through [`read_keyring`], asserts it matches, then cleans up. Ignored by
/// default — requires a working OS keychain backend (macOS Keychain / Linux
/// Secret Service / Windows Credential Manager). On a headless Linux box
/// without `gnome-keyring`/`kwallet` this will fail at the write step.
///
/// Run with: cargo test --features backend-http,keyring --test keyring -- --ignored live_keyring_roundtrip
#[test]
#[ignore]
fn live_keyring_roundtrip() {
    use proserpina::backend::credentials::{read_keyring, KEYRING_SERVICE};
    let var = "DEEPSEEK_API_KEY";
    let sentinel = "proserpina-test-sentinel-key";
    // Clean up any leftover entry first.
    let _ = keyring::Entry::new(KEYRING_SERVICE, var).and_then(|e| e.delete_credential());

    let entry = keyring::Entry::new(KEYRING_SERVICE, var).expect("create entry");
    entry.set_password(sentinel).expect("write sentinel");

    let read = read_keyring(var).expect("read_keyring should succeed");
    assert_eq!(read.as_deref(), Some(sentinel));

    // Clean up.
    entry.delete_credential().expect("cleanup");
}
