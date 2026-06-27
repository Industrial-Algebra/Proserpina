# The Document and the Transcript: `Subject`, `Transcript`

## `Subject`

```rust
pub struct Subject { /* text: String, source: Option<String> */ }

impl Subject {
    pub fn from_markdown(text: impl Into<String>, source: impl AsRef<str>) -> Self;
    pub fn text(&self) -> &str;
    pub fn source(&self) -> Option<&str>;
}
```

The document under critique. v1 holds opaque markdown text plus an optional
source path (empty source → `None`, so anonymous documents round-trip
cleanely). The runner broadcasts `subject.text()` to critics as a `Prompt`.

## `Transcript`

```rust
pub struct Transcript { /* Vec<Message> */ }

impl Transcript {
    pub fn new() -> Self;
    pub fn push(&mut self, message: Message);
    pub fn iter(&self) -> impl Iterator<Item = &Message>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

The ordered record of `Message`s produced during a run. `Runner::execute`
appends to a transcript as it walks the graph; the [summarizer](./summarizer.md)
reads it to produce [findings](./report.md).
