# Critics and Panels: `Persona`, `Panel`

```rust
pub struct Persona { /* name, framing?, focus? */ }

impl Persona {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn with_framing(self, ...) -> Self;
    pub fn with_focus(self, ...) -> Self;
    pub fn name(&self) -> &str;
    pub fn framing(&self) -> Option<&str>;
    pub fn focus(&self) -> Option<&str>;
    pub fn archetypes() -> &'static [Persona];   // the 5 built-ins
    pub fn default_panel() -> Vec<Persona>;       // single Devil's Advocate
}

pub enum Panel { Default, Duo, Panel }

impl Panel {
    pub fn personas(&self) -> Vec<Persona>;
    pub fn from_name(name: &str) -> Option<Self>;  // case-insensitive
    pub fn name(&self) -> &'static str;
}

pub fn resolve_panel(name: &str, credentials: &Credentials) -> Result<Vec<Persona>, ProserpinaError>;
```

A `Persona` is the lens a critic applies — **data, not an enum** — so critics
are configurable without code changes. The five archetypes (Devil's Advocate,
Methodologist, Red Team, Domain Expert, Editor) back the built-in `Panel`
presets (`default` / `duo` / `panel`).

`resolve_panel` resolves a name to personas: a `[panels.NAME]` config section
overrides a same-named built-in; otherwise the built-in; otherwise
[`UnknownPanel`](./errors.md). See [Panels](../guide/panels.md).
