// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Daemon-style credential resolution: pure, no filesystem, no env reads,
//! no keyring, no CLI. This is the surface Ijima-style consumers use.

#![cfg(feature = "credentials")]

use proserpina_agent::credentials::{resolve_configs, Credentials, Provider};
use std::collections::HashMap;

#[test]
fn resolves_authed_config_from_explicit_env_keys() {
    let registry = Provider::registry();
    let creds = Credentials::from_toml("").unwrap();
    let mut env = HashMap::new();
    env.insert("DEEPSEEK_API_KEY".to_string(), "sk-test".to_string());

    let configs = resolve_configs(registry, &creds, &env).unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].api_key, "sk-test");
    assert!(!configs[0].model.is_empty());
    assert!(!configs[0].base_url.is_empty());
}

#[test]
fn no_keys_yields_no_configs() {
    let registry = Provider::registry();
    let creds = Credentials::from_toml("").unwrap();
    let configs = resolve_configs(registry, &creds, &HashMap::new()).unwrap();
    assert!(configs.is_empty());
}

#[test]
fn config_override_beats_registry_default_model() {
    let registry = Provider::registry();
    let creds = Credentials::from_toml(
        r#"
[openai]
api_key = "sk-from-config"
model = "gpt-5.5"
"#,
    )
    .unwrap();
    let configs = resolve_configs(registry, &creds, &HashMap::new()).unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].model, "gpt-5.5");
    assert_eq!(configs[0].api_key, "sk-from-config");
}
