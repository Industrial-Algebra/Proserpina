// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for agent-readiness: provider attribution, capabilities,
//! dry-run plan, structured errors + exit codes.

#![cfg(feature = "backend-http")]

use praxis::backend::http::{HttpAgent, HttpConfig};
use praxis::{Agent, AgentId, Capabilities, Message, MessageKind, Persona, Plan, ProviderInfo};

#[test]
fn http_agent_error_names_the_model_not_just_the_persona() {
    // Provider attribution (the 429-debugging fix): an HttpAgent failure must
    // identify the provider/model, not just the persona, so a multi-provider
    // run can tell which one died.
    let mut agent = HttpAgent::new(
        AgentId::new("Devil's Advocate"),
        Persona::new("Devil's Advocate"),
        HttpConfig {
            base_url: "https://nonexistent-host-invalid.invalid/v1".to_owned(),
            model: "deepseek-chat".to_owned(),
            api_key: "dummy".to_owned(),
        },
    );
    let prompt = Message::new(
        AgentId::new("system"),
        Some(AgentId::new("Devil's Advocate")),
        MessageKind::Prompt,
        "doc",
    );
    let err = agent.respond(&prompt).expect_err("bad host should fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("deepseek-chat"),
        "error should name the model; got: {msg}"
    );
    assert!(
        msg.contains("Devil's Advocate"),
        "error should still name the persona; got: {msg}"
    );
}

// ---- Capabilities ----

#[test]
fn capabilities_reports_version_subcommands_formats_and_topologies() {
    let caps = Capabilities::static_info();
    assert!(!caps.version.is_empty(), "version should be set");
    assert!(caps.subcommands.contains(&"critique".to_owned()));
    assert!(caps.subcommands.contains(&"capabilities".to_owned()));
    assert!(caps.output_formats.contains(&"markdown".to_owned()));
    assert!(caps.output_formats.contains(&"json".to_owned()));
    assert!(caps.topologies.contains(&"parallel".to_owned()));
    assert!(caps.topologies.contains(&"rounds".to_owned()));
}

#[test]
fn capabilities_provider_list_marks_authed_vs_unauthed() {
    // With no keys in the env and no config, no registry provider is authed.
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
    std::env::set_var("PRAXIS_CONFIG", "/nonexistent/praxis-test-cap.toml");
    let caps = Capabilities::with_current_auth();
    std::env::remove_var("PRAXIS_CONFIG");

    // Six registry providers reported.
    assert!(caps.providers.len() >= 6, "registry providers present");
    // None authed in this stripped environment.
    assert!(
        caps.providers.iter().all(|p| !p.authed),
        "no keys set -> none authed"
    );
    // Each provider has a name and model.
    for p in &caps.providers {
        assert!(!p.name.is_empty());
        assert!(!p.model.is_empty());
    }
}

#[test]
fn capabilities_serializes_to_json() {
    let caps = Capabilities::static_info();
    let json = serde_json::to_string(&caps).expect("serializes");
    assert!(json.contains("\"version\""));
    assert!(json.contains("\"providers\""));
    assert!(json.contains("\"exit_codes\""));
}

// ---- Plan / dry-run ----

fn cfg(tag: &str) -> HttpConfig {
    HttpConfig {
        base_url: format!("https://{tag}.invalid/v1"),
        model: format!("{tag}-model"),
        api_key: format!("key-{tag}"),
    }
}

#[test]
fn plan_reports_roster_seed_and_call_counts_without_network() {
    let personas = Persona::default_panel();
    let configs = vec![cfg("deepseek"), cfg("openai")];
    let plan = Plan::for_parallel(&personas, &configs, 42);

    assert_eq!(plan.seed, 42);
    assert_eq!(plan.topology, "parallel");
    assert_eq!(plan.roster.len(), personas.len());
    // Each slot carries a persona + provider + model.
    for slot in &plan.roster {
        assert!(!slot.persona.is_empty());
        assert!(!slot.provider.is_empty());
        assert!(!slot.model.is_empty());
    }
    // One critic call per persona, plus one summarizer call.
    assert_eq!(plan.n_critic_calls, personas.len());
    assert_eq!(plan.n_summarizer_calls, 1);
    assert_eq!(plan.estimated_total_calls, personas.len() + 1);
}

#[test]
fn plan_for_parallel_is_deterministic_given_seed() {
    let personas = Persona::default_panel();
    let configs = vec![cfg("a"), cfg("b"), cfg("c")];
    let p1 = Plan::for_parallel(&personas, &configs, 7);
    let p2 = Plan::for_parallel(&personas, &configs, 7);
    let models1: Vec<_> = p1.roster.iter().map(|s| s.model.clone()).collect();
    let models2: Vec<_> = p2.roster.iter().map(|s| s.model.clone()).collect();
    assert_eq!(models1, models2, "same seed -> same plan");
}

#[test]
fn plan_serializes_to_json() {
    let plan = Plan::for_parallel(&Persona::default_panel(), &[cfg("a")], 1);
    let json = serde_json::to_string(&plan).expect("serializes");
    assert!(json.contains("\"seed\""));
    assert!(json.contains("\"roster\""));
    assert!(json.contains("\"estimated_total_calls\""));
}

// ---- PraxisError: exit codes + error JSON ----

use praxis::PraxisError;

#[test]
fn exit_code_maps_each_variant_to_a_distinct_value() {
    assert_eq!(PraxisError::no_authed_providers(vec![]).exit_code(), 10);
    assert_eq!(PraxisError::agent_failure("x", "y").exit_code(), 11);
    assert_eq!(PraxisError::summary_failed("x").exit_code(), 12);
    // MissingAgent is 15 (not the 70 fallback).
    assert_eq!(
        PraxisError::missing_agent(AgentId::new("g")).exit_code(),
        15
    );
}

#[test]
fn error_kind_is_a_stable_machine_string() {
    assert_eq!(
        PraxisError::no_authed_providers(vec![]).error_kind(),
        "no_authed_providers"
    );
    assert_eq!(
        PraxisError::agent_failure("x", "y").error_kind(),
        "agent_failure"
    );
    assert_eq!(
        PraxisError::summary_failed("x").error_kind(),
        "summary_failed"
    );
}

#[test]
fn to_error_json_carries_kind_message_and_details() {
    let err = PraxisError::no_authed_providers(vec!["deepseek".to_owned(), "openai".to_owned()]);
    let json = err.to_error_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
    assert_eq!(parsed["error"]["kind"], "no_authed_providers");
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("deepseek"));
    // Structured details for machine consumers.
    assert_eq!(parsed["error"]["details"]["tried"][0], "deepseek");
}
