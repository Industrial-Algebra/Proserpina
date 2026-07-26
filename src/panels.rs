// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Built-in critic panel presets — a critique-pipeline concern.
//!
//! The core [`Persona`] type lives in `proserpina-agent`; the named panel
//! presets (`default` / `duo` / `panel`) and config-aware resolution live
//! here because they are critique-pipeline concepts (daemon consumers like
//! Ijima define their own personas and do not use panels).

use crate::persona::Persona;
#[cfg(feature = "backend-http")]
use crate::{backend::credentials::Credentials, error::ProserpinaError};

/// A built-in named panel preset.
///
/// `Default` is the single-Devil's-Advocate panel (back-compat); `Duo` adds
/// the Methodologist; `Panel` is the full five-critic cross-examination panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Single Devil's Advocate (the historical default).
    Default,
    /// Devil's Advocate + Methodologist.
    Duo,
    /// All five archetypes.
    Panel,
}

impl Panel {
    /// The personas in this preset, in canonical order.
    pub fn personas(&self) -> Vec<Persona> {
        let archetypes = Persona::archetypes();
        match self {
            Panel::Default => vec![archetypes[0].clone()],
            Panel::Duo => vec![archetypes[0].clone(), archetypes[1].clone()],
            Panel::Panel => archetypes.to_vec(),
        }
    }

    /// Parses a built-in panel by name (case-insensitive).
    ///
    /// Returns `None` for unknown names so the caller can fall through to
    /// config-defined panels.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "default" => Some(Panel::Default),
            "duo" => Some(Panel::Duo),
            "panel" => Some(Panel::Panel),
            _ => None,
        }
    }

    /// The canonical name of this preset.
    pub fn name(&self) -> &'static str {
        match self {
            Panel::Default => "default",
            Panel::Duo => "duo",
            Panel::Panel => "panel",
        }
    }
}

/// Resolves a panel name to its personas.
///
/// Order: a config-defined panel under `[panels.NAME]` overrides a same-named
/// built-in; otherwise built-in presets (`default`/`duo`/`panel`) are used; an
/// unknown name yields [`ProserpinaError::UnknownPanel`] listing what *was*
/// available (built-ins + config sections).
///
/// Pure given the credentials config — unit-testable without env or IO.
///
/// # Errors
///
/// Returns [`ProserpinaError::UnknownPanel`] if `name` is neither built-in nor in
/// `credentials.panels()`.
#[cfg(feature = "backend-http")]
pub fn resolve_panel(
    name: &str,
    credentials: &Credentials,
) -> Result<Vec<Persona>, ProserpinaError> {
    // 1. Config-defined panel (overrides built-in of the same name).
    if let Some(panel) = credentials.panels().get(name) {
        return Ok(panel.personas.iter().map(|s| s.to_persona()).collect());
    }

    // 2. Built-in preset.
    if let Some(preset) = Panel::from_name(name) {
        return Ok(preset.personas());
    }

    // 3. Unknown — list what was available.
    let mut available: Vec<String> = vec!["default", "duo", "panel"]
        .into_iter()
        .map(String::from)
        .collect();
    for n in credentials.panels().keys() {
        available.push(n.clone());
    }
    Err(ProserpinaError::unknown_panel(name, available))
}
