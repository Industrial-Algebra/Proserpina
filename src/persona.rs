// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Critic personas — the lens an agent applies when critiquing.

/// A `Persona` is data, not an enum: a name, an optional framing sentence, and
/// an optional focus area.
///
/// Treating personas as data lets users configure critics without code changes.
/// A built-in registry of archetypes (Devil's Advocate, Methodologist, Red
/// Teamer, Domain Expert, Editor) lands alongside the runner; v1 ships the type
/// and its builder so the data model is complete and testable in isolation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Persona {
    name: String,
    framing: Option<String>,
    focus: Option<String>,
}

impl Persona {
    /// Creates a persona with just a name.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::Persona;
    /// let p = Persona::new("Devil's Advocate");
    /// assert_eq!(p.name(), "Devil's Advocate");
    /// assert_eq!(p.framing(), None);
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            framing: None,
            focus: None,
        }
    }

    /// Sets the framing sentence: how the persona is instructed to approach
    /// the document.
    #[must_use]
    pub fn with_framing(mut self, framing: impl Into<String>) -> Self {
        self.framing = Some(framing.into());
        self
    }

    /// Sets the focus area: what the persona is asked to pay attention to.
    #[must_use]
    pub fn with_focus(mut self, focus: impl Into<String>) -> Self {
        self.focus = Some(focus.into());
        self
    }

    /// The persona's human-readable name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The framing sentence, if set.
    pub fn framing(&self) -> Option<&str> {
        self.framing.as_deref()
    }

    /// The focus area, if set.
    pub fn focus(&self) -> Option<&str> {
        self.focus.as_deref()
    }

    /// The default critic panel used when none is configured: a single Devil's
    /// Advocate. Lives in `persona` so it is available without the `cli`
    /// feature; the CLI delegates here.
    pub fn default_panel() -> Vec<Persona> {
        vec![Persona::new("Devil's Advocate")
            .with_framing("Assume the proposal is wrong; find how.")
            .with_focus("logical gaps and unsupported assumptions")]
    }

    /// The built-in archetype personas, in canonical order: Devil's Advocate,
    /// Methodologist, Red Team, Domain Expert, Editor.
    ///
    /// These back the built-in [`Panel`] presets (`duo`, `panel`) and are
    /// available as data for user-defined panels.
    pub fn archetypes() -> &'static [Persona] {
        static ARCHETYPES: std::sync::OnceLock<Vec<Persona>> = std::sync::OnceLock::new();
        ARCHETYPES.get_or_init(|| {
            vec![
                Persona::new("Devil's Advocate")
                    .with_framing("Assume the proposal is wrong; find how.")
                    .with_focus("logical gaps and unsupported assumptions"),
                Persona::new("Methodologist")
                    .with_framing("Scrutinize the rigor of every claim.")
                    .with_focus("proof gaps and methodological soundness"),
                Persona::new("Red Team")
                    .with_framing("Find how this fails in practice.")
                    .with_focus("failure modes and adversarial conditions"),
                Persona::new("Domain Expert")
                    .with_framing("Evaluate against the domain state of the art.")
                    .with_focus("technical accuracy and novelty"),
                Persona::new("Editor")
                    .with_framing("Improve clarity and structure.")
                    .with_focus("readability and missing context"),
            ]
        })
    }
}

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
    credentials: &crate::backend::credentials::Credentials,
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

#[cfg(feature = "backend-http")]
use crate::error::ProserpinaError;
