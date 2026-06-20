// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

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

/// How serious a [`Finding`] is.
///
/// Ordered `Blocker > Major > Minor > Info` so findings can be sorted by
/// severity. Exhaustive on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

/// A single synthesized critique of the subject.
#[derive(Debug, Clone)]
pub struct Finding {
    author: AgentId,
    severity: Severity,
    summary: String,
}

impl Finding {
    /// Creates a new finding from its parts.
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::{AgentId, Finding, Severity};
    /// let f = Finding::new(
    ///     AgentId::new("critic-a"),
    ///     Severity::Major,
    ///     "Assumptions unsupported.",
    /// );
    /// assert_eq!(f.author().as_str(), "critic-a");
    /// ```
    pub fn new(author: AgentId, severity: Severity, summary: impl Into<String>) -> Self {
        Self {
            author,
            severity,
            summary: summary.into(),
        }
    }

    /// The critic that produced this finding.
    pub fn author(&self) -> &AgentId {
        &self.author
    }

    /// How serious this finding is.
    pub fn severity(&self) -> &Severity {
        &self.severity
    }

    /// A one-line summary of the finding (the critique text, for the echo backend).
    pub fn summary(&self) -> &str {
        &self.summary
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
                author: m.sender().clone(),
                severity: Severity::Major,
                summary: m.text().to_owned(),
            })
            .collect();
        Self { findings }
    }

    /// The findings in this report, in synthesis order.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Renders the report as markdown, including a header noting the source
    /// document (if known).
    pub fn to_markdown_with_source(&self, source: Option<&str>) -> String {
        let mut out = String::from("# Critique Report\n\n");
        if let Some(src) = source {
            out.push_str(&format!("**Subject:** `{src}`\n\n"));
        }
        if self.findings.is_empty() {
            out.push_str("No findings.\n");
            return out;
        }
        for (i, finding) in self.findings.iter().enumerate() {
            out.push_str(&format!(
                "## {}. [{}] {}\n\n",
                i + 1,
                finding.severity.label(),
                finding.summary,
            ));
            out.push_str(&format!("_— {}_\n\n", finding.author));
        }
        out
    }

    /// Renders the report as markdown.
    pub fn to_markdown(&self) -> String {
        self.to_markdown_with_source(None)
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
