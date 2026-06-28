// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! The document under critique.

/// A `Subject` is whatever a Proserpina run is cross-examining.
///
/// v1 holds opaque markdown text plus an optional source path. Later, an
/// extraction stage may break a `Subject` into claims or sections; the graph
/// operates on whichever units are present, so the core type stays simple.
#[derive(Debug, Clone)]
pub struct Subject {
    text: String,
    source: Option<String>,
}

impl Subject {
    /// Builds a `Subject` from markdown text and a source path.
    ///
    /// An empty source path is treated as "unknown" and stored as `None`, so
    /// anonymous documents round-trip cleanly.
    ///
    /// # Examples
    ///
    /// ```
    /// use proserpina::Subject;
    /// let s = Subject::from_markdown("# Plan\n\nbody", "roadmap.md");
    /// assert_eq!(s.text(), "# Plan\n\nbody");
    /// assert_eq!(s.source(), Some("roadmap.md"));
    /// ```
    pub fn from_markdown(text: impl Into<String>, source: impl AsRef<str>) -> Self {
        let source = source.as_ref();
        let source = if source.is_empty() {
            None
        } else {
            Some(source.to_owned())
        };
        Self {
            text: text.into(),
            source,
        }
    }

    /// The full markdown text of the document.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The originating path or URL of the document, if known.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}
