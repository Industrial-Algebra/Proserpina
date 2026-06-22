// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the multi-provider roster.
//!
//! The roster splits cleanly into a pure core (`random_roster`) and an
//! env-touching layer (`Provider::config_from_env`, `roster_from_env`).
//! These tests cover both, with env reads isolated to dedicated tests using
//! unique variable names so they don't collide with the caller's environment.

#![cfg(feature = "backend-http")]

use praxis::backend::roster::{random_roster, roster_from_env, Provider};
use praxis::Persona;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn provider_exposes_its_fields() {
    let p = Provider::new("deepseek")
        .with_base_url("https://api.deepseek.com/v1")
        .with_model("deepseek-chat")
        .with_key_env_var("DEEPSEEK_API_KEY");
    assert_eq!(p.name(), "deepseek");
    assert_eq!(p.base_url(), "https://api.deepseek.com/v1");
    assert_eq!(p.model(), "deepseek-chat");
    assert_eq!(p.key_env_var(), "DEEPSEEK_API_KEY");
}

#[test]
fn config_from_env_returns_none_when_key_unset() {
    // Use a var name nobody would plausibly set, then make sure it's absent.
    let var = "PRAXIS_TEST_DEFINITELY_UNSET_KEY";
    std::env::remove_var(var);
    let p = Provider::new("test")
        .with_base_url("https://example.invalid/v1")
        .with_model("m")
        .with_key_env_var(var);
    assert!(p.config_from_env().is_none());
}

#[test]
fn config_from_env_builds_config_when_key_set() {
    let var = "PRAXIS_TEST_PROVIDER_KEY";
    std::env::set_var(var, "sk-test-value");
    let p = Provider::new("test")
        .with_base_url("https://example.invalid/v1")
        .with_model("test-model")
        .with_key_env_var(var);
    let cfg = p.config_from_env().expect("key was just set");
    std::env::remove_var(var); // clean up for other tests / the environment

    assert_eq!(cfg.base_url, "https://example.invalid/v1");
    assert_eq!(cfg.model, "test-model");
    assert_eq!(cfg.api_key, "sk-test-value");
}

#[test]
fn registry_contains_the_known_frontier_providers() {
    let names: Vec<&str> = Provider::registry().iter().map(|p| p.name()).collect();
    for expected in ["deepseek", "openai", "moonshot", "alibaba", "zai", "google"] {
        assert!(
            names.contains(&expected),
            "registry should include {expected}, got {names:?}"
        );
    }
}

#[test]
fn registry_entries_are_fully_populated() {
    // Every preset must have a non-empty base_url, model, and key_env_var —
    // a half-filled preset would silently produce a broken HttpConfig.
    for p in Provider::registry() {
        assert!(!p.base_url().is_empty(), "{} has empty base_url", p.name());
        assert!(!p.model().is_empty(), "{} has empty model", p.name());
        assert!(
            !p.key_env_var().is_empty(),
            "{} has empty key_env_var",
            p.name()
        );
    }
}

// ---- random_roster: pure core ----

fn cfg(tag: &str) -> praxis::backend::http::HttpConfig {
    praxis::backend::http::HttpConfig {
        base_url: format!("https://{tag}.invalid/v1"),
        model: format!("{tag}-model"),
        api_key: format!("key-{tag}"),
    }
}

fn personas(n: usize) -> Vec<Persona> {
    (0..n)
        .map(|i| Persona::new(format!("critic-{i}")))
        .collect()
}

#[test]
fn random_roster_is_deterministic_given_a_seed() {
    let configs = vec![cfg("a"), cfg("b"), cfg("c")];
    let ps = personas(4);

    let r1 = random_roster(&ps, &configs, &mut StdRng::seed_from_u64(42));
    let r2 = random_roster(&ps, &configs, &mut StdRng::seed_from_u64(42));
    assert_eq!(r1.len(), r2.len());
    for (a, b) in r1.iter().zip(r2.iter()) {
        assert_eq!(a.1.model, b.1.model, "same seed -> same model assignment");
    }
}

#[test]
fn random_roster_different_seeds_usually_differ() {
    // Different seeds should (almost always) produce different assignments
    // for a non-trivial panel. We compare the model sequence.
    let configs = vec![cfg("a"), cfg("b"), cfg("c"), cfg("d")];
    let ps = personas(6);

    let r1: Vec<String> = random_roster(&ps, &configs, &mut StdRng::seed_from_u64(1))
        .iter()
        .map(|(_, c)| c.model.clone())
        .collect();
    let r2: Vec<String> = random_roster(&ps, &configs, &mut StdRng::seed_from_u64(2))
        .iter()
        .map(|(_, c)| c.model.clone())
        .collect();
    assert_ne!(r1, r2, "different seeds should yield different assignments");
}

#[test]
fn random_roster_assigns_only_from_the_provided_configs() {
    let configs = vec![cfg("a"), cfg("b")];
    let allowed: Vec<String> = configs.iter().map(|c| c.model.clone()).collect();
    let roster = random_roster(&personas(5), &configs, &mut StdRng::seed_from_u64(7));

    for (_, c) in &roster {
        assert!(
            allowed.contains(&c.model),
            "roster assigned a model not in the input set: {}",
            c.model
        );
    }
}

#[test]
fn random_roster_preserves_persona_count_and_order() {
    let configs = vec![cfg("a"), cfg("b")];
    let ps = personas(3);
    let roster = random_roster(&ps, &configs, &mut StdRng::seed_from_u64(99));

    assert_eq!(roster.len(), 3);
    let names: Vec<&str> = roster.iter().map(|(p, _)| p.name()).collect();
    assert_eq!(names, vec!["critic-0", "critic-1", "critic-2"]);
}

#[test]
fn random_roster_with_no_configs_produces_empty_roster() {
    let roster = random_roster(&personas(3), &[], &mut StdRng::seed_from_u64(0));
    assert!(roster.is_empty(), "no authed configs -> no agents");
}

#[test]
fn random_roster_with_no_personas_produces_empty_roster() {
    let roster = random_roster(&[], &[cfg("a")], &mut StdRng::seed_from_u64(0));
    assert!(roster.is_empty());
}

// ---- roster_from_env: pipeline + error path ----

#[test]
fn roster_from_env_errors_when_no_providers_are_authed() {
    // A registry of providers whose key vars are all unset.
    let var_a = "PRAXIS_TEST_ROSTER_UNSET_A";
    let var_b = "PRAXIS_TEST_ROSTER_UNSET_B";
    std::env::remove_var(var_a);
    std::env::remove_var(var_b);
    let providers = vec![
        Provider::new("a")
            .with_base_url("https://a.invalid/v1")
            .with_model("ma")
            .with_key_env_var(var_a),
        Provider::new("b")
            .with_base_url("https://b.invalid/v1")
            .with_model("mb")
            .with_key_env_var(var_b),
    ];

    let result = roster_from_env(&personas(2), &providers, 1);
    assert!(result.is_err(), "no keys set -> should error");
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.to_lowercase().contains("no") || msg.to_lowercase().contains("auth"));
}

#[test]
fn roster_from_env_uses_only_authed_providers() {
    // One authed, one not. The roster should draw only from the authed one,
    // regardless of how many personas there are.
    let var_authed = "PRAXIS_TEST_ROSTER_AUTHED";
    let var_unauthed = "PRAXIS_TEST_ROSTER_UNAUTHED";
    std::env::set_var(var_authed, "sk-authed");
    std::env::remove_var(var_unauthed);
    let providers = vec![
        Provider::new("authed")
            .with_base_url("https://authed.invalid/v1")
            .with_model("authed-model")
            .with_key_env_var(var_authed),
        Provider::new("unauthed")
            .with_base_url("https://unauthed.invalid/v1")
            .with_model("unauthed-model")
            .with_key_env_var(var_unauthed),
    ];

    let roster = roster_from_env(&personas(4), &providers, 42).expect("one provider is authed");
    std::env::remove_var(var_authed);

    assert_eq!(roster.len(), 4);
    for (_, c) in &roster {
        assert_eq!(
            c.model, "authed-model",
            "should only have drawn from the authed provider"
        );
    }
}
