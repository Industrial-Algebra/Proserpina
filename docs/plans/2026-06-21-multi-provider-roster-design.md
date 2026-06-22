# Praxis — Multi-Provider Roster Design

- **Date:** 2026-06-21
- **Status:** Approved (design phase; implementation via TDD)
- **Branch:** `feature/multi-provider-roster`
- **Depends on:** `backend-http` feature (PR #6)

## 1. Purpose

Praxis's HTTP backend reaches any OpenAI-compatible provider. Justin has
frontier-model access across DeepSeek, Z.ai (GLM), OpenAI, Google, Moonshot,
and Alibaba. Model diversity improves cross-examination: different frontier
models have different blind spots, biases, and strengths, so a panel that
mixes them catches what a homogeneous panel misses.

This design adds a **provider preset registry** and a **random roster
builder** that assigns authed providers to critic personas — leveraging that
diversity automatically, while staying reproducible and fully unit-testable.

## 2. Key Design Decisions

1. **No new transport.** All six providers expose OpenAI-compatible endpoints,
   so the existing `HttpAgent` + `HttpConfig` already reach them. This is a
   *configuration + assignment* layer, not a new backend.
2. **Pure roster function; Runner stays pure.** Provider assignment is a
   standalone, seedable, unit-testable function that produces
   `Vec<(Persona, HttpConfig)>`; the caller (CLI) feeds the results to
   `Runner::with_agent(HttpAgent::new(...))`. The Runner is untouched,
   matching the IA "separate policy from execution" pattern.
3. **Provider is data, not an enum** (like `Persona`). Adding providers later
   is data, not code. A built-in registry ships the six frontier presets.
4. **Two-layer split for testability.** `Provider::config_from_env()` is the
   only env-touching piece; `random_roster` is pure given the authed configs,
   so it is unit-testable with hand-built configs + a seeded RNG.
5. **Always-seeded RNG.** Random by default; reproducible on demand. If
   `--seed` is omitted, Praxis generates one from entropy and prints it in the
   report header so every run is re-runnable exactly.

## 3. Data Model

### `Provider` preset

```rust
pub struct Provider {
    pub name: String,        // "deepseek"
    pub base_url: String,    // "https://api.deepseek.com/v1"
    pub model: String,       // "deepseek-chat"
    pub key_env_var: String, // "DEEPSEEK_API_KEY"
}
```

- `Provider::config_from_env(&self) -> Option<HttpConfig>` — reads the env var;
  returns `None` if unset (provider not authed).
- `Provider::registry() -> &'static [Provider]` — the six built-in presets:

| name | base_url | model | key_env_var |
|---|---|---|---|
| deepseek | `https://api.deepseek.com/v1` | `deepseek-chat` | `DEEPSEEK_API_KEY` |
| openai | `https://api.openai.com/v1` | `gpt-4o` | `OPENAI_API_KEY` |
| moonshot | `https://api.moonshot.cn/v1` | `moonshot-v1-auto` | `MOONSHOT_API_KEY` |
| alibaba | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` | `DASHSCOPE_API_KEY` |
| zai | `https://open.bigmodel.cn/api/paas/v4` | `glm-4-plus` | `ZAI_API_KEY` |
| google | `https://generativelanguage.googleapis.com/v1beta/openai` | `gemini-1.5-pro` | `GOOGLE_API_KEY` |

> Model strings drift over time (e.g. `glm-5.2` today). The presets are a
> sensible default; exact model names are verified/updated during
> implementation and are not load-bearing to the design.

### `random_roster` (pure core)

```rust
pub fn random_roster(
    personas: &[Persona],
    configs: &[HttpConfig],
    rng: &mut impl rand::Rng,
) -> Vec<(Persona, HttpConfig)>
```

- For each persona (in order), independently picks a random config from
  `configs` and pairs it with a clone of the persona.
- Returns one entry per persona (preserving count and order); empty if either
  input is empty.
- **Deterministic** given the RNG state. Independent random per persona (two
  critics may share a model) — simpler than forced shuffle, and persona- plus
  model-diversity together is the goal.

### `roster_from_env` (CLI convenience)

```rust
pub fn roster_from_env(
    personas: &[Persona],
    providers: &[Provider],
    seed: u64,
) -> Result<Vec<(Persona, HttpConfig)>, PraxisError>
```

Builds the authed configs via `config_from_env`, seeds an RNG from `seed`, and
calls `random_roster`. Errors `NoAuthedProviders` when zero keys are set.

### Error

`PraxisError::NoAuthedProviders` — "no API keys found in the environment for
any registered provider."

## 4. Crate Shape

- New module `src/backend/roster.rs`, gated `#[cfg(feature = "backend-http")]`
  (it references `HttpConfig`, so it is an HTTP concern).
- `rand` added behind the `backend-http` feature. Default build pulls in zero
  new deps.
- Surfaced as `praxis::backend::roster::{Provider, random_roster, roster_from_env}`.

## 5. CLI Integration

- `run_critique` switches from the hardcoded Devil's Advocate to
  `roster_from_env` over `Provider::registry()`.
- New `--seed <N>` flag (optional). If omitted, generate from entropy and
  print the chosen seed in the report header so the run is reproducible.
- `praxis critique doc.md --seed 42` reproduces an earlier run exactly.

## 6. Implementation Sequencing (TDD)

Each step RED → GREEN → REFACTOR, every public item documented.

1. **`Provider` + `config_from_env`** — struct, accessors, env-reading method
   (tested with a unique env var name; None when unset, Some when set).
2. **`Provider::registry()`** — the six presets (test: contains known names,
   every entry has non-empty fields).
3. **`random_roster` pure core** — determinism (same seed → identical output),
   every output config drawn from the input set, persona count/order
   preserved, empty-input edge cases.
4. **`roster_from_env` + `NoAuthedProviders`** — full pipeline; errors when
   no providers authed, succeeds + uses only authed providers when some are.
5. **CLI wiring** — `--seed` flag, always-seeded, seed printed in report,
   uses `roster_from_env`. Existing echo-backed CLI test stays green.

## 7. Open Questions / Future Work

- **Forced-shuffle mode** (one provider per persona max) as an alternative
  assignment policy for small panels.
- **Persona-declared model preference** — let a persona name a preferred
  provider (Methodologist → most rigorous), falling back to random.
- **Per-provider retry/timeout** (see PR #6 deferred items) — more important
  once a run fans out across six providers with different rate limits.
- **Live multi-provider smoke test** analogous to `examples/deepseek_smoke.rs`,
  parameterized over authed providers.
