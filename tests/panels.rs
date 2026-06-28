// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for configurable persona panels.

#![cfg(feature = "backend-http")]

use proserpina::backend::credentials::Credentials;
use proserpina::persona::{resolve_panel, Panel};
use proserpina::Persona;

#[test]
fn archetypes_returns_the_five_built_in_personas() {
    let archetypes = Persona::archetypes();
    let names: Vec<&str> = archetypes.iter().map(|p| p.name()).collect();
    assert_eq!(
        names,
        vec![
            "Devil's Advocate",
            "Methodologist",
            "Red Team",
            "Domain Expert",
            "Editor",
        ]
    );
}

#[test]
fn archetypes_are_fully_specified_with_framing_and_focus() {
    // Every archetype must carry framing and focus, not just a name — a
    // half-specified persona would produce a weak prompt.
    for p in Persona::archetypes() {
        assert!(!p.name().is_empty(), "archetype has a name");
        assert!(p.framing().is_some(), "{} has framing", p.name());
        assert!(p.focus().is_some(), "{} has focus", p.name());
    }
}

// ---- built-in Panel presets ----

#[test]
fn panel_default_is_a_single_devils_advocate() {
    let personas = Panel::Default.personas();
    assert_eq!(personas.len(), 1);
    assert_eq!(personas[0].name(), "Devil's Advocate");
}

#[test]
fn panel_duo_is_devils_advocate_plus_methodologist() {
    let personas = Panel::Duo.personas();
    assert_eq!(personas.len(), 2);
    assert_eq!(personas[0].name(), "Devil's Advocate");
    assert_eq!(personas[1].name(), "Methodologist");
}

#[test]
fn panel_panel_is_all_five_archetypes() {
    let personas = Panel::Panel.personas();
    assert_eq!(personas.len(), 5);
    assert_eq!(
        personas.iter().map(|p| p.name()).collect::<Vec<_>>(),
        Persona::archetypes()
            .iter()
            .map(|p| p.name())
            .collect::<Vec<_>>()
    );
}

#[test]
fn panel_from_name_round_trips_for_built_ins() {
    for name in ["default", "duo", "panel"] {
        let panel = Panel::from_name(name).expect("built-in parses");
        assert_eq!(panel.name(), name);
    }
}

#[test]
fn panel_from_name_is_case_insensitive_and_rejects_unknown() {
    assert!(Panel::from_name("DUO").is_some());
    assert!(Panel::from_name("Panel").is_some());
    assert!(Panel::from_name("nonexistent").is_none());
}

// ---- resolve_panel (built-in + config + error) ----

#[test]
fn resolve_panel_returns_built_in_presets() {
    let creds = Credentials::default();
    assert_eq!(resolve_panel("default", &creds).unwrap().len(), 1);
    assert_eq!(resolve_panel("duo", &creds).unwrap().len(), 2);
    assert_eq!(resolve_panel("panel", &creds).unwrap().len(), 5);
}

#[test]
fn resolve_panel_reads_a_user_defined_panel_from_config() {
    let toml = r#"
[panels.red-team]
personas = [
  { name = "Skeptic", framing = "Doubt everything.", focus = "assumptions" },
  { name = "Nitpicker", framing = "Find the small flaws.", focus = "details" },
]
"#;
    let creds = Credentials::from_toml(toml).expect("valid toml");
    let personas = resolve_panel("red-team", &creds).expect("panel exists");
    assert_eq!(personas.len(), 2);
    assert_eq!(personas[0].name(), "Skeptic");
    assert_eq!(personas[0].framing(), Some("Doubt everything."));
    assert_eq!(personas[1].name(), "Nitpicker");
    assert_eq!(personas[1].focus(), Some("details"));
}

#[test]
fn resolve_panel_errors_on_unknown_name_with_available_listed() {
    let toml = r#"
[panels.red-team]
personas = [{ name = "Skeptic" }]
"#;
    let creds = Credentials::from_toml(toml).expect("valid toml");
    let err = resolve_panel("nope", &creds).expect_err("unknown -> error");
    let msg = format!("{err}");
    assert!(msg.contains("nope"), "error names the requested panel");
    // The available panels (built-in + config) should be listed so the message
    // is actionable.
    assert!(msg.contains("default"));
    assert!(msg.contains("duo"));
    assert!(msg.contains("panel"));
    assert!(msg.contains("red-team"));
}

#[test]
fn resolve_panel_user_panel_overrides_built_in_name() {
    // A config section named "default" overrides the built-in default panel.
    let toml = r#"
[panels.default]
personas = [{ name = "Custom" }]
"#;
    let creds = Credentials::from_toml(toml).expect("valid toml");
    let personas = resolve_panel("default", &creds).unwrap();
    assert_eq!(personas.len(), 1);
    assert_eq!(personas[0].name(), "Custom");
}

// ---- capabilities surfaces panels ----

#[test]
fn capabilities_lists_available_panels() {
    let caps = proserpina::Capabilities::static_info();
    assert!(caps.panels.contains(&"default".to_owned()));
    assert!(caps.panels.contains(&"duo".to_owned()));
    assert!(caps.panels.contains(&"panel".to_owned()));
}
