// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! The report: synthesized findings and markdown rendering.
//!
//! A [`Report`] is produced by folding a [`Transcript`](crate::transcript::Transcript):
//! every [`MessageKind::Critique`] becomes a [`Finding`]. Prompts, questions,
//! and other kinds are skipped — only critiques are findings. (Later
//! topologies may fold rebuttals and concessions into richer findings; v1
//! keeps the rule simple and unambiguous.)

use crate::agent::AgentId;
use crate::message::MessageKind;
use crate::transcript::Transcript;
use std::cmp::Reverse;

/// How serious a [`Finding`] is.
///
/// Ordered `Blocker > Major > Minor > Info` so findings can be sorted by
/// severity. Exhaustive on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Severity {
    /// A neutral observation or note.
    Info,
    /// A minor issue worth flagging.
    Minor,
    /// A substantive problem that should be addressed.
    Major,
    /// A problem that blocks acceptance of the document as-is.
    Blocker,
}

/// A single synthesized critique finding: one distinct issue raised (and
/// possibly corroborated) across the critic panel.
///
/// Findings are produced either by the simple
/// [`Report::from_transcript`] fold (one per `Critique` message, `Major`, no
/// extra fields) or by the summarizer (clustered across critics with the full
/// rich field set). Both paths populate the same type.
#[derive(Debug, Clone)]
pub struct Finding {
    /// How serious this finding is.
    pub severity: Severity,
    /// A freeform category, e.g. "methodology", "falsifiability".
    pub category: Option<String>,
    /// The issue, in one line.
    pub summary: String,
    /// Where in the subject the issue lives, e.g. "§2", "line 47".
    pub location: Option<String>,
    /// The excerpt of the subject being critiqued.
    pub quote: Option<String>,
    /// An actionable recommended change.
    pub suggested_change: Option<String>,
    /// The critics that raised or agreed with this finding (clustered).
    supporting_critics: Vec<AgentId>,
}

impl Finding {
    /// Creates a minimal finding with a severity and summary.
    ///
    /// All optional fields start empty; `supporting_critics` starts empty.
    /// Populate the rest with the `with_*` builders.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::{Finding, Severity};
    /// let f = Finding::new(Severity::Major, "Unsupported assumption.");
    /// assert_eq!(f.summary(), "Unsupported assumption.");
    /// ```
    pub fn new(severity: Severity, summary: impl Into<String>) -> Self {
        Self {
            severity,
            category: None,
            summary: summary.into(),
            location: None,
            quote: None,
            suggested_change: None,
            supporting_critics: Vec::new(),
        }
    }

    /// Sets the category.
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Sets the category only if `category` is `Some`.
    #[must_use]
    pub fn with_category_opt(mut self, category: Option<String>) -> Self {
        if let Some(c) = category {
            self.category = Some(c);
        }
        self
    }

    /// Sets the location.
    #[must_use]
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Sets the location only if `location` is `Some`.
    #[must_use]
    pub fn with_location_opt(mut self, location: Option<String>) -> Self {
        if let Some(l) = location {
            self.location = Some(l);
        }
        self
    }

    /// Sets the quote.
    #[must_use]
    pub fn with_quote(mut self, quote: impl Into<String>) -> Self {
        self.quote = Some(quote.into());
        self
    }

    /// Sets the quote only if `quote` is `Some`.
    #[must_use]
    pub fn with_quote_opt(mut self, quote: Option<String>) -> Self {
        if let Some(q) = quote {
            self.quote = Some(q);
        }
        self
    }

    /// Sets the suggested change.
    #[must_use]
    pub fn with_suggested_change(mut self, change: impl Into<String>) -> Self {
        self.suggested_change = Some(change.into());
        self
    }

    /// Sets the suggested change only if `change` is `Some`.
    #[must_use]
    pub fn with_suggested_change_opt(mut self, change: Option<String>) -> Self {
        if let Some(c) = change {
            self.suggested_change = Some(c);
        }
        self
    }

    /// Sets the supporting critics.
    #[must_use]
    pub fn with_supporting_critics(mut self, critics: Vec<AgentId>) -> Self {
        self.supporting_critics = critics;
        self
    }

    /// How serious this finding is.
    pub fn severity(&self) -> &Severity {
        &self.severity
    }

    /// The issue, in one line.
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The critics that raised or agreed with this finding.
    pub fn supporting_critics(&self) -> &[AgentId] {
        &self.supporting_critics
    }

    /// The first supporting critic, if any. Convenience for the echo-backend
    /// path (one author per finding) and back-compat with older callers.
    pub fn author(&self) -> Option<&AgentId> {
        self.supporting_critics.first()
    }
}

/// A synthesized critique report: the findings folded from a transcript.
#[derive(Debug, Clone, Default)]
pub struct Report {
    findings: Vec<Finding>,
}

impl Report {
    /// Creates an empty report.
    pub fn new() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    /// Appends a finding to the report.
    pub fn push_finding(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Synthesizes a report from a transcript.
    ///
    /// Every [`MessageKind::Critique`] in the transcript becomes a [`Finding`],
    /// in transcript order. All other message kinds are skipped. The echo
    /// backend cannot infer severity, so synthesized findings default to
    /// [`Severity::Major`]; real backends will supply richer findings later.
    pub fn from_transcript(transcript: &Transcript) -> Self {
        let findings = transcript
            .iter()
            .filter(|m| matches!(m.kind(), MessageKind::Critique))
            .map(|m| Finding {
                severity: Severity::Major,
                category: None,
                summary: m.text().to_owned(),
                location: None,
                quote: None,
                suggested_change: None,
                supporting_critics: vec![m.sender().clone()],
            })
            .collect();
        Self { findings }
    }

    /// The findings in this report, in synthesis order.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Renders the report as a human-readable digest, including a header
    /// noting the source document (if known) and an executive summary of
    /// counts by severity, followed by findings sorted by severity desc.
    pub fn to_markdown_with_source(&self, source: Option<&str>) -> String {
        let mut out = String::from("# Critique Report\n\n");
        if let Some(src) = source {
            out.push_str(&format!("**Subject:** `{src}`\n\n"));
        }
        if self.findings.is_empty() {
            out.push_str("No findings.\n");
            return out;
        }
        // Executive summary: counts by severity.
        out.push_str(&format!(
            "**Findings:** {} ({} blocker, {} major, {} minor, {} info)\n\n",
            self.findings.len(),
            self.count(Severity::Blocker),
            self.count(Severity::Major),
            self.count(Severity::Minor),
            self.count(Severity::Info),
        ));
        // Findings sorted by severity desc (stable for ties).
        let mut sorted: Vec<&Finding> = self.findings.iter().collect();
        sorted.sort_by_key(|f| Reverse(f.severity));
        for (i, finding) in sorted.iter().enumerate() {
            out.push_str(&format!(
                "## {}. [{}] {}\n\n",
                i + 1,
                finding.severity.label(),
                finding.summary,
            ));
            if let Some(cat) = &finding.category {
                out.push_str(&format!("- **Category:** {cat}\n"));
            }
            if let Some(loc) = &finding.location {
                out.push_str(&format!("- **Location:** {loc}\n"));
            }
            if let Some(quote) = &finding.quote {
                out.push_str(&format!("- **Quote:** > {quote}\n"));
            }
            if let Some(change) = &finding.suggested_change {
                out.push_str(&format!("- **Suggested change:** {change}\n"));
            }
            if !finding.supporting_critics.is_empty() {
                let names: Vec<&str> = finding
                    .supporting_critics
                    .iter()
                    .map(|c| c.as_str())
                    .collect();
                out.push_str(&format!("- **Raised by:** {}\n", names.join(", ")));
            }
            out.push('\n');
        }
        out
    }

    /// Counts findings at a given severity.
    fn count(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }

    /// Renders the report as markdown.
    pub fn to_markdown(&self) -> String {
        self.to_markdown_with_source(None)
    }

    /// Renders the report as JSON (requires the `json` feature).
    ///
    /// Emits the findings sorted by severity desc, matching the markdown
    /// digest order. The same `Vec<Finding>` underlies both renders.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> String {
        let mut sorted: Vec<&Finding> = self.findings.iter().collect();
        sorted.sort_by_key(|f| Reverse(f.severity));
        serde_json::to_string_pretty(&JsonReport {
            findings: sorted.iter().map(|f| JsonFinding::from(*f)).collect(),
        })
        .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_owned())
    }
}

impl Severity {
    /// A lowercase label suitable for rendering.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Minor => "minor",
            Severity::Major => "major",
            Severity::Blocker => "blocker",
        }
    }
}

// ---- JSON render helpers (behind the `json` feature) ----

#[cfg(feature = "json")]
#[derive(serde::Serialize)]
struct JsonReport<'a> {
    findings: Vec<JsonFinding<'a>>,
}

#[cfg(feature = "json")]
#[derive(serde::Serialize)]
struct JsonFinding<'a> {
    severity: &'a Severity,
    category: &'a Option<String>,
    summary: &'a str,
    location: &'a Option<String>,
    quote: &'a Option<String>,
    suggested_change: &'a Option<String>,
    supporting_critics: Vec<&'a str>,
}

#[cfg(feature = "json")]
impl<'a> From<&'a Finding> for JsonFinding<'a> {
    fn from(f: &'a Finding) -> Self {
        Self {
            severity: &f.severity,
            category: &f.category,
            summary: &f.summary,
            location: &f.location,
            quote: &f.quote,
            suggested_change: &f.suggested_change,
            supporting_critics: f.supporting_critics.iter().map(|c| c.as_str()).collect(),
        }
    }
}
