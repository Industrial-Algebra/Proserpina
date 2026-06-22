// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the credentials config.
//!
//! The config splits into a pure core (`Credentials::from_toml`,
//! `resolve_providers`) and an I/O layer (`from_path`/`discover`). These tests
//! cover both; env reads are isolated to an explicit snapshot in
//! `resolve_providers`, so no test mutates the real environment.

#![cfg(feature = "backend-http")]

use std::collections::HashMap;

use praxis::backend::credentials::{
    authed_configs, resolve_configs, Credentials, ProviderOverride,
};
use praxis::backend::http::HttpConfig;
use praxis::backend::roster::Provider;

#[test]
fn credentials_parse_a_minimal_single_provider() {
    let toml = r#"
[deepseek]
api_key = "sk-abc"
"#;
    let creds = Credentials::from_toml(toml).expect("valid toml");
    let ds = creds.override_for("deepseek").expect("deepseek present");
    assert_eq!(ds.api_key.as_deref(), Some("sk-abc"));
    assert!(ds.model.is_none());
    assert!(ds.base_url.is_none());
}

#[test]
fn credentials_parse_provider_with_model_and_base_url_overrides() {
    let toml = r#"
[zai]
api_key = "k"
model = "glm-5.2"
base_url = "https://custom.example/v1"
"#;
    let creds = Credentials::from_toml(toml).expect("valid toml");
    let zai = creds.override_for("zai").expect("zai present");
    assert_eq!(zai.api_key.as_deref(), Some("k"));
    assert_eq!(zai.model.as_deref(), Some("glm-5.2"));
    assert_eq!(zai.base_url.as_deref(), Some("https://custom.example/v1"));
}

#[test]
fn credentials_parse_multiple_providers_and_custom_entries() {
    let toml = r#"
[deepseek]
api_key = "ds"

[my-local-llm]
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama"
"#;
    let creds = Credentials::from_toml(toml).expect("valid toml");
    assert!(creds.override_for("deepseek").is_some());
    let custom = creds.override_for("my-local-llm").expect("custom present");
    assert_eq!(
        custom.base_url.as_deref(),
        Some("http://localhost:11434/v1")
    );
    assert_eq!(custom.model.as_deref(), Some("llama3"));
    assert_eq!(custom.api_key.as_deref(), Some("ollama"));
}

#[test]
fn credentials_from_str_errors_on_malformed_toml() {
    let bad = r#"
[deepseek
api_key = missing-quotes
"#;
    assert!(Credentials::from_toml(bad).is_err());
}

#[test]
fn empty_credentials_have_no_overrides() {
    let creds = Credentials::from_toml("").expect("empty toml is valid");
    assert!(creds.override_for("anything").is_none());
}

#[test]
fn from_path_reads_a_written_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("credentials.toml");
    std::fs::write(
        &path,
        r#"
[deepseek]
api_key = "from-file"

[zai]
api_key = "k"
model = "glm-5.2"
"#,
    )
    .expect("write");

    let creds = Credentials::from_path(&path).expect("file parses");
    assert_eq!(
        creds.override_for("deepseek").unwrap().api_key.as_deref(),
        Some("from-file")
    );
    assert_eq!(
        creds.override_for("zai").unwrap().model.as_deref(),
        Some("glm-5.2")
    );
}

#[test]
fn from_path_errors_on_a_nonexistent_path() {
    let path = std::path::PathBuf::from("/nonexistent/praxis/credentials.toml");
    assert!(Credentials::from_path(&path).is_err());
}

#[test]
fn from_path_errors_on_a_malformed_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "[deepseek\napi_key = oops").expect("write");
    assert!(Credentials::from_path(&path).is_err());
}

// ---- resolve_configs: pure resolution core ----

fn env(keys: &[(&str, &str)]) -> HashMap<String, String> {
    keys.iter()
        .copied()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

fn registry() -> Vec<Provider> {
    // A small fake registry mirroring the shape of Provider::registry().
    vec![
        Provider::new("deepseek")
            .with_base_url("https://api.deepseek.com/v1")
            .with_model("deepseek-chat")
            .with_key_env_var("DEEPSEEK_API_KEY"),
        Provider::new("zai")
            .with_base_url("https://open.bigmodel.cn/api/paas/v4")
            .with_model("glm-4-plus")
            .with_key_env_var("ZAI_API_KEY"),
    ]
}

fn find<'a>(configs: &'a [HttpConfig], model: &str) -> Option<&'a HttpConfig> {
    configs.iter().find(|c| c.model == model)
}

#[test]
fn resolve_returns_only_providers_with_a_resolved_key() {
    // deepseek key in env; zai has none -> only deepseek resolves.
    let env = env(&[("DEEPSEEK_API_KEY", "sk-env")]);
    let creds = Credentials::default();
    let configs = resolve_configs(&registry(), &creds, &env).expect("ok");
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].model, "deepseek-chat");
    assert_eq!(configs[0].api_key, "sk-env");
}

#[test]
fn resolve_env_key_takes_precedence_over_config_key() {
    // Both env and config have a deepseek key -> env wins.
    let env = env(&[("DEEPSEEK_API_KEY", "sk-env")]);
    let creds = Credentials::from_toml(
        r#"[deepseek]
api_key = "sk-config""#,
    )
    .unwrap();
    let configs = resolve_configs(&registry(), &creds, &env).unwrap();
    assert_eq!(configs[0].api_key, "sk-env");
}

#[test]
fn resolve_config_key_used_when_env_absent() {
    let env = HashMap::new();
    let creds = Credentials::from_toml(
        r#"[deepseek]
api_key = "sk-config""#,
    )
    .unwrap();
    let configs = resolve_configs(&registry(), &creds, &env).unwrap();
    assert_eq!(configs[0].api_key, "sk-config");
}

#[test]
fn resolve_applies_config_model_override_over_registry_default() {
    // zai config overrides the drifted glm-4-plus with glm-5.2.
    let env = env(&[("ZAI_API_KEY", "k")]);
    let creds = Credentials::from_toml(
        r#"[zai]
api_key = "k"
model = "glm-5.2""#,
    )
    .unwrap();
    let configs = resolve_configs(&registry(), &creds, &env).unwrap();
    let zai = find(&configs, "glm-5.2").expect("glm-5.2 should be present");
    assert_eq!(zai.base_url, "https://open.bigmodel.cn/api/paas/v4");
}

#[test]
fn resolve_applies_config_base_url_override() {
    let env = env(&[("DEEPSEEK_API_KEY", "k")]);
    let creds = Credentials::from_toml(
        r#"[deepseek]
api_key = "k"
base_url = "https://proxy.example/v1""#,
    )
    .unwrap();
    let configs = resolve_configs(&registry(), &creds, &env).unwrap();
    assert_eq!(configs[0].base_url, "https://proxy.example/v1");
}

#[test]
fn resolve_includes_custom_providers_not_in_the_registry() {
    let env = HashMap::new();
    let creds = Credentials::from_toml(
        r#"[my-local-llm]
base_url = "http://localhost:11434/v1"
model = "llama3"
api_key = "ollama""#,
    )
    .unwrap();
    let configs = resolve_configs(&registry(), &creds, &env).unwrap();
    let custom = find(&configs, "llama3").expect("custom provider resolves");
    assert_eq!(custom.base_url, "http://localhost:11434/v1");
    assert_eq!(custom.api_key, "ollama");
}

#[test]
fn resolve_errors_when_a_custom_provider_is_missing_required_fields() {
    let env = HashMap::new();
    // Missing base_url and model.
    let creds = Credentials::from_toml(
        r#"[my-broken]
api_key = "k""#,
    )
    .unwrap();
    let result = resolve_configs(&registry(), &creds, &env);
    let err = result.expect_err("missing fields -> error");
    let msg = format!("{err}");
    assert!(msg.contains("my-broken"), "error should name the provider");
    assert!(msg.contains("model"));
    assert!(msg.contains("base_url"));
}

#[test]
fn resolve_custom_provider_env_fallback_uses_uppercased_name() {
    // A custom provider with no api_key in config falls back to
    // <UPPERCASE_NAME>_API_KEY in the env snapshot.
    let env = env(&[("MY_LOCAL_LLM_API_KEY", "sk-env")]);
    let creds = Credentials::from_toml(
        r#"[my-local-llm]
base_url = "http://x/v1"
model = "m""#,
    )
    .unwrap();
    let configs = resolve_configs(&registry(), &creds, &env).unwrap();
    assert_eq!(configs[0].api_key, "sk-env");
}

#[test]
fn resolve_returns_empty_when_nothing_authed_and_no_custom() {
    let env = HashMap::new();
    let creds = Credentials::default();
    let configs = resolve_configs(&registry(), &creds, &env).unwrap();
    assert!(configs.is_empty());
}

#[test]
fn authed_configs_uses_resolve_over_real_registry_and_env() {
    // authed_configs() is the thin wrapper over the real env + registry.
    // With the standard provider keys unset and no config file, it should
    // produce an empty vec (nothing authed) — not an error. Erroring on
    // empty is the caller's job (the CLI's run_critique).
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
    // Point discovery at a nonexistent file so no real config interferes.
    std::env::set_var("PRAXIS_CONFIG", "/nonexistent/praxis-test-credentials.toml");
    let configs = authed_configs().expect("empty is not an error");
    std::env::remove_var("PRAXIS_CONFIG");
    assert!(
        configs.is_empty(),
        "no keys + no config -> no authed configs"
    );
}
