// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Messages: the edges of the interaction graph.

use crate::agent::AgentId;

/// The role a [`Message`] plays in a critique.
///
/// Exhaustive on purpose: every variant must be handled by the runner and the
/// synthesizer, and adding one is a conscious decision (not a silent fall-through).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// The subject broadcast to critics — a prompt to critique, not itself a
    /// finding. Distinct from [`MessageKind::Critique`] so the synthesizer can
    /// fold critiques while skipping prompts unambiguously.
    Prompt,
    /// A substantive critique of the subject or another message.
    Critique,
    /// A counter-argument to a prior [`MessageKind::Critique`].
    Rebuttal,
    /// A clarifying question.
    Question,
    /// An explicit concession — the sender withdraws or softens a prior point.
    Concession,
    /// A final adjudication, typically produced by a moderator topology.
    Verdict,
}

impl MessageKind {
    /// A stable lowercase label for the variant.
    ///
    /// Intended as a serialization key (and serde tag once the `serde` feature
    /// lands); kept stable so external tooling can rely on it.
    pub fn label(&self) -> &'static str {
        match self {
            MessageKind::Prompt => "prompt",
            MessageKind::Critique => "critique",
            MessageKind::Rebuttal => "rebuttal",
            MessageKind::Question => "question",
            MessageKind::Concession => "concession",
            MessageKind::Verdict => "verdict",
        }
    }

    /// Parses a label produced by [`MessageKind::label`].
    ///
    /// # Errors
    ///
    /// Returns `Err` with the unrecognized label if `text` is not a known kind.
    pub fn from_label(text: &str) -> Result<Self, &'static str> {
        match text {
            "prompt" => Ok(MessageKind::Prompt),
            "critique" => Ok(MessageKind::Critique),
            "rebuttal" => Ok(MessageKind::Rebuttal),
            "question" => Ok(MessageKind::Question),
            "concession" => Ok(MessageKind::Concession),
            "verdict" => Ok(MessageKind::Verdict),
            other => {
                let _ = other;
                Err("unknown message kind")
            }
        }
    }
}

/// A single message routed along an edge of the interaction graph.
///
/// `sender` is always present; `recipient` is `None` for broadcast messages
/// (e.g. a critique addressed to all other critics, or the initial subject
/// prompt addressed to the whole panel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    sender: AgentId,
    recipient: Option<AgentId>,
    kind: MessageKind,
    text: String,
}

impl Message {
    /// Creates a new message.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::{AgentId, Message, MessageKind};
    /// let msg = Message::new(
    ///     AgentId::new("critic-a"),
    ///     Some(AgentId::new("critic-b")),
    ///     MessageKind::Critique,
    ///     "Unsupported assumption.",
    /// );
    /// assert_eq!(msg.sender().as_str(), "critic-a");
    /// ```
    pub fn new(
        sender: AgentId,
        recipient: Option<AgentId>,
        kind: MessageKind,
        text: impl Into<String>,
    ) -> Self {
        Self {
            sender,
            recipient,
            kind,
            text: text.into(),
        }
    }

    /// Who sent this message.
    pub fn sender(&self) -> &AgentId {
        &self.sender
    }

    /// Who this message is addressed to, if not a broadcast.
    pub fn recipient(&self) -> Option<&AgentId> {
        self.recipient.as_ref()
    }

    /// The role this message plays.
    pub fn kind(&self) -> MessageKind {
        self.kind
    }

    /// The message body.
    pub fn text(&self) -> &str {
        &self.text
    }
}
