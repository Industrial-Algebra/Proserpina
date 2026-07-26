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
    /// use proserpina_agent::Persona;
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
    /// These back the built-in panel presets (`duo`, `panel`) in the
    /// `proserpina` critique pipeline and are available as data for
    /// user-defined panels.
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

