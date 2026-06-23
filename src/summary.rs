// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The summarizer: structures a critique transcript into rich [`Finding`]s.
//!
//! After a run, Praxis makes a second LLM call (the *summarizer*) over the
//! whole transcript + subject, asking the model to group the critiques into
//! distinct issues and emit each as a fenced ` ```praxis-finding ` block. This
//! module renders that prompt and parses the response.
//!
//! ## Graceful degradation
//!
//! [`parse_findings`] never fails on a malformed response: an unrecognized
//! severity defaults to `Major`, an unparseable block is skipped, and a
//! response with no blocks yields an empty `Vec`. A run only fails if the
//! summarizer *call* itself fails (network/HTTP error → `SummaryFailed`).

use crate::agent::AgentId;
use crate::backend::http::HttpConfig;
use crate::error::PraxisError;
use crate::message::MessageKind;
use crate::report::{Finding, Severity};
use crate::subject::Subject;
use crate::transcript::Transcript;

/// The fenced-block tag the summarizer emits for each finding.
pub(crate) const FINDING_BLOCK_TAG: &str = "praxis-finding";

/// Parses a summarizer response body into [`Finding`]s.
///
/// Extracts every ` ```praxis-finding ` fenced block and parses its
/// `key: value` lines into a `Finding`. Graceful degradation:
/// - Unrecognized `severity` → `Major`.
/// - Missing optional fields → `None` / empty.
/// - A block with no `summary` line → skipped.
/// - No blocks at all → empty `Vec`.
///
/// # Examples
///
/// ```
/// use praxis::summary::parse_findings;
/// let body = "```praxis-finding\nseverity: major\nsummary: X.\n```";
/// let findings = parse_findings(body);
/// assert_eq!(findings.len(), 1);
/// ```
pub fn parse_findings(body: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for block in extract_finding_blocks(body) {
        if let Some(f) = parse_block(&block) {
            findings.push(f);
        }
    }
    findings
}

/// Extracts the inner text of each ` ```praxis-finding ` fenced block.
fn extract_finding_blocks(body: &str) -> Vec<String> {
    let open = format!("```{FINDING_BLOCK_TAG}");
    let mut blocks = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(&open) {
        let after_open = &rest[start + open.len()..];
        // The fence may have trailing tokens on its line; skip to the newline.
        let line_end = after_open.find('\n').unwrap_or(after_open.len());
        let after_open_line = &after_open[line_end..];
        let Some(close_offset) = after_open_line.find("```") else {
            break; // unterminated fence; stop
        };
        let block_inner = &after_open_line[..close_offset];
        blocks.push(block_inner.trim().to_owned());
        rest = &after_open_line[close_offset + 3..];
    }
    blocks
}

/// Parses one block's inner text (key: value lines) into a `Finding`.
fn parse_block(block: &str) -> Option<Finding> {
    let mut severity: Option<Severity> = None;
    let mut category: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut location: Option<String> = None;
    let mut quote: Option<String> = None;
    let mut suggested_change: Option<String> = None;
    let mut supporting: Vec<AgentId> = Vec::new();

    for line in block.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "severity" => severity = Some(parse_severity(value)),
            "category" => category = Some(value.to_owned()),
            "summary" => summary = Some(value.to_owned()),
            "location" => location = Some(value.to_owned()),
            "quote" => quote = Some(value.to_owned()),
            "suggested_change" => suggested_change = Some(value.to_owned()),
            "supporting_critics" => {
                supporting = value
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(AgentId::new)
                    .collect();
            }
            _ => {}
        }
    }

    let summary = summary?;
    Some(
        Finding::new(severity.unwrap_or(Severity::Major), summary)
            .with_category_opt(category)
            .with_location_opt(location)
            .with_quote_opt(quote)
            .with_suggested_change_opt(suggested_change)
            .with_supporting_critics(supporting),
    )
}

fn parse_severity(value: &str) -> Severity {
    match value.to_lowercase().as_str() {
        "info" => Severity::Info,
        "minor" => Severity::Minor,
        "major" => Severity::Major,
        "blocker" => Severity::Blocker,
        _ => Severity::Major, // graceful default
    }
}

/// Renders the summarizer's system/user prompt from a subject + transcript.
///
/// The system message instructs the model to group critiques into distinct
/// issues and emit a fenced `praxis-finding` block per issue. The user message
/// carries the subject text and the transcript turns.
pub fn render_summary_prompt(subject: &Subject, transcript: &Transcript) -> Vec<SummaryMessage> {
    let system = "You are summarizing a multi-critic peer review. Group the \
critiques into distinct issues. Emit ONE fenced code block per issue, tagged \
```praxis-finding```, with these fields (one per line, `key: value`):\n\
severity: (info|minor|major|blocker)\n\
category: (a short label)\n\
summary: (the issue, one line)\n\
location: (where in the document, e.g. §2 or line 47)\n\
quote: (the excerpt being critiqued)\n\
suggested_change: (an actionable recommended change)\n\
supporting_critics: (comma-separated critic names)\n\n\
All fields except `severity` and `summary` are optional. Cluster issues that \
multiple critics raised; list them in supporting_critics. Do not emit prose \
outside the blocks.";

    let mut user = String::new();
    user.push_str("# Document under critique\n\n");
    user.push_str(subject.text());
    user.push_str("\n\n# Critique transcript\n\n");
    for msg in transcript.iter() {
        user.push_str(&format!(
            "[{}] ({}) {}: {}\n",
            msg.sender(),
            kind_label(msg.kind()),
            msg.kind().label(),
            msg.text(),
        ));
    }

    vec![
        SummaryMessage {
            role: "system".to_owned(),
            content: system.to_owned(),
        },
        SummaryMessage {
            role: "user".to_owned(),
            content: user,
        },
    ]
}

fn kind_label(_kind: MessageKind) -> &'static str {
    // Turn labels into human words for the summarizer's prompt.
    "critic"
}

/// A chat message in the summarizer conversation (mirrors the HTTP backend's
/// `ChatMessage`, kept separate so this module is self-contained).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SummaryMessage {
    /// `system` or `user`.
    pub role: String,
    /// The message content.
    pub content: String,
}

/// Runs the summarizer: renders the prompt for `subject` + `transcript`, calls
/// an OpenAI-compatible chat-completions endpoint via `config`, and parses the
/// response into [`Finding`]s.
///
/// Holds a dedicated Tokio runtime to bridge the synchronous call site to
/// async HTTP (same pattern as `HttpAgent`). Graceful on parse: an empty or
/// malformed response yields an empty `Vec`, not an error. Only fails if the
/// HTTP call itself fails.
///
/// # Errors
///
/// Returns [`PraxisError::SummaryFailed`] if the HTTP request fails or the
/// response body cannot be read.
pub fn summarize(
    subject: &Subject,
    transcript: &Transcript,
    config: &HttpConfig,
) -> Result<Vec<Finding>, PraxisError> {
    let messages = render_summary_prompt(subject, transcript);
    let body = serde_json::json!({
        "model": config.model,
        "messages": messages,
    });
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| PraxisError::summary_failed(format!("runtime build: {e}")))?;

    let client = reqwest::Client::new();
    let body_text = runtime.block_on(async {
        let resp = client
            .post(&url)
            .bearer_auth(&config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| PraxisError::summary_failed(format!("HTTP send: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| PraxisError::summary_failed(format!("HTTP body: {e}")))?;
        if !status.is_success() {
            return Err(PraxisError::summary_failed(format!(
                "HTTP {status}: {text}"
            )));
        }
        Ok::<String, PraxisError>(text)
    })?;

    // Extract choices[0].message.content (same shape as the HTTP backend).
    let parsed: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| PraxisError::summary_failed(format!("invalid JSON: {e}")))?;
    let content = parsed
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| PraxisError::summary_failed("response had no choices[0].message.content"))?;

    Ok(parse_findings(content))
}
