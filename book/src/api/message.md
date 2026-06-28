# Messages: `Message`, `MessageKind`

```rust
pub enum MessageKind {
    Prompt,       // the subject broadcast to critics
    Critique,     // a substantive finding
    Rebuttal,     // a counter-argument to a Critique
    Question,     // a clarifying question
    Concession,   // withdrawal/softening of a prior point
    Verdict,      // a final adjudication (moderated topology)
}

pub struct Message {
    /* sender: AgentId, recipient: Option<AgentId>, kind: MessageKind, text: String */
}

impl Message {
    pub fn new(sender: AgentId, recipient: Option<AgentId>,
               kind: MessageKind, text: impl Into<String>) -> Self;
    pub fn sender(&self) -> &AgentId;
    pub fn recipient(&self) -> Option<&AgentId>;
    pub fn kind(&self) -> MessageKind;
    pub fn text(&self) -> &str;
}
```

`Message` is the edge payload of the interaction graph. `sender` is always
present; `recipient` is `None` for broadcasts. `MessageKind` is exhaustive on
purpose — the runner and synthesizer handle every variant, and the report
synthesizer folds `Critique`s into findings while skipping `Prompt`s.

`MessageKind` round-trips through a stable lowercase `label()` / `from_label()`
for future serde use.

See [the interaction graph](./graph.md) for how messages route.
