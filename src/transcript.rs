// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The transcript: the ordered record of messages produced by a run.

use crate::message::Message;

/// An ordered record of the [`Message`]s produced during a Praxis run.
///
/// A [`Runner`](crate::runner::Runner) appends to a `Transcript` as it walks
/// the interaction graph. The transcript is the raw material the report
/// synthesizer folds into findings.
#[derive(Debug, Clone, Default)]
pub struct Transcript {
    messages: Vec<Message>,
}

impl Transcript {
    /// Creates a new, empty transcript.
    ///
    /// # Examples
    ///
    /// ```
    /// use praxis::Transcript;
    /// let t = Transcript::new();
    /// assert!(t.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Appends a message to the end of the transcript.
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// The number of messages recorded so far.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the transcript contains no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Iterates over the recorded messages, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter()
    }
}
