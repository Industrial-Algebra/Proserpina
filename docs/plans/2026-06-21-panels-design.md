# Praxis — Configurable Persona Panels Design

- **Date:** 2026-06-21
- **Status:** Approved (design phase; implementation via TDD)
- **Branch:** `feature/configurable-panels`
- **Depends on:** roster (#7), credentials (#8), agent-readiness (#10)
- **Motivation:** Every live Praxis demo so far ran a single Devil's Advocate.
  The roster's multi-model diversity value is unrealized until a run can be
  N critics × M providers. This is the biggest unrealized value in the
  codebase.

## 1. Purpose

Let a run use a configurable **panel** of N critic personas. Each persona is
assigned a provider via the existing roster, so a 5-critic panel fanned across
DeepSeek/Z.ai/OpenAI/etc. produces a genuinely diverse cross-examination — the
core value Praxis was designed for.

## 2. Key Design Decisions

1. **Same config file.** Panels live in the existing `credentials.toml` as a
   `[panels.NAME]` section. Justin already has that file for providers; no new
   file to manage. (Separate file would split config unnecessarily.)
2. **Built-in named panels.** `default` (single Devil's Advocate — current
   behavior, unchanged), `duo` (2), `panel` (5 archetypes). Common cases need
   zero config; user panels extend/override.
3. **`default` stays the default.** All existing tests, demos, and the
   `run_critique`/`run_critique_echo`/`plan_critique` paths keep using
   `default_panel()` unless `--panel` is given. Back-compat preserved.
4. **`Panel::resolve` is pure** given the config. Built-in lookup first, then
   config sections. New error `UnknownPanel`.
5. **Agent-discoverable.** `praxis capabilities` gains a `panels` field
   listing available panels (built-in + config-defined), so an agent learns
   what's on offer without reading docs.

## 3. Built-in Archetypes

The `panel` archetype uses five critics (from the original design's persona
registry):

| Persona | Framing | Focus |
|---|---|---|
| Devil's Advocate | Assume the proposal is wrong; find how. | logical gaps, unsupported assumptions |
| Methodologist | Scrutinize the rigor of every claim. | proof gaps, methodological soundness |
| Red Team | Find how this fails in practice. | failure modes, adversarial conditions |
| Domain Expert | Evaluate against domain state-of-the-art. | technical accuracy, novelty |
| Editor | Improve clarity and structure. | readability, missing context |

`duo` = Devil's Advocate + Methodologist. `default` = Devil's Advocate only.

## 4. Config Format

```toml
# Named panel, defined inline in credentials.toml:
[panels.red-team]
personas = [
  { name = "Devil's Advocate", framing = "...", focus = "..." },
  { name = "Methodologist",    framing = "...", focus = "proof rigor" },
  { name = "Red Team",         framing = "...", focus = "failure modes" },
]
```

A `[panels.NAME]` section with a `personas` array. Each persona is
`{ name, framing?, focus? }`. Names must be unique within a panel (the roster
keys agents by `AgentId` derived from persona name).

## 5. API Surface

```rust
impl Persona {
    /// The built-in archetypes (Devil's Advocate, Methodologist, Red Team,
    /// Domain Expert, Editor).
    pub fn archetypes() -> &'static [&'static Persona] { ... }
}

/// Resolves a panel name to its personas.
///
/// Built-ins (`default`, `duo`, `panel`) first; then config `[panels.NAME]`.
pub fn resolve_panel(
    name: &str,
    credentials: &Credentials,
) -> Result<Vec<Persona>, PraxisError>;
```

New error: `PraxisError::UnknownPanel { name, available: Vec<String> }` —
carries the names that *were* available, so the message and the agent's error
JSON are actionable.

## 6. CLI Integration

- `praxis critique doc.md --panel red-team` — uses the named panel.
- `praxis critique doc.md --panel panel` — uses the 5-critic built-in.
- Omitted `--panel` → `default` (unchanged behavior).
- `praxis capabilities` gains a `panels: Vec<String>` field listing built-in
  + config-defined panel names.

## 7. Implementation Sequencing (TDD)

Each step RED → GREEN → REFACTOR.

1. **`Persona::archetypes()`** — the five built-in personas as data.
2. **Built-in panels** — `default`/`duo`/`panel` resolve to `Vec<Persona>`.
3. **`[panels]` config parse + `resolve_panel`** — built-in lookup, config
   lookup, `UnknownPanel` error (with available names).
4. **CLI `--panel` flag + capabilities `panels` field.**
5. **Live multi-critic run** to prove the diversity value.

## 8. Open Questions / Future Work

- **Per-persona provider pinning** — let a persona name a preferred provider
  (Methodologist → most rigorous model). Deferred; random assignment first.
- **Panel inheritance/composition** — `[panels.X] extends = "panel"`. YAGNI
  for now.
- **Persona presets as a separate concern** — currently archetypes are
  in-code; a `[personas.NAME]` config section could let users define reusable
  personas referenced by multiple panels. Deferred.
