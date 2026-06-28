# Critique of the Praxis v0.1.0 Release

> **v0.1.0 Snapshot** — This is an honest self-assessment of Praxis at its
> initial public release. It records the project's weak points and open
> questions as of v0.1.0, kept public as a benchmark for future improvement.
> Several items below have known follow-ups tracked in
> [ROADMAP.md](./ROADMAP.md).

## Overview

Praxis is a Rust CLI + library that runs a panel of LLM-backed critic personas
over a document and summarizes their critiques into actionable findings. The
architecture is clean (provider-agnostic `Agent` trait; pure roster/panel
resolution; sync engine with a contained sync/async bridge), the test suite is
solid (136 tests, including end-to-end live runs), and the agent-discoverability
surface is genuinely good. The honest weaknesses are below.

---

## Key Criticisms

1. **Single-critic default; multi-critic is opt-in.**
   The `default` panel is a single Devil's Advocate. The roster's
   multi-provider diversity value — Praxis's headline feature — is only
   realized when the user reaches for `--panel duo` or `--panel panel`. New
   users running `praxis critique doc.md` with defaults get a competent but
   *single-model* critique and may not realize what they're missing. The
   quick-start docs surface `--panel panel` early, but the default itself
   under-sells the tool.

2. **Plaintext keys without the `keyring` feature; Linux even with it.**
   v0.1.0 ships an OS keychain tier (the `keyring` feature, highest-precedence
   source) that works on macOS Keychain and Windows Credential Manager. But
   it's opt-in (not in the default install), and on Linux with gnome-keyring
   the `keyring` crate's backend has a write-Ok/read-NoEntry quirk that makes
   it unreliable. So the *default* install path still stores keys in plaintext
   (`~/.config/praxis/credentials.toml`) with no permissions check. Real
   improvement over a pure-plaintext world, but not a complete fix — tracked
   in ROADMAP.

3. **Cost is invisible until it's incurred.**
   `--dry-run` shows call counts, but Praxis doesn't estimate USD cost, and
   there's no spend guard. A `--panel panel` run with retry is 6+ frontier
   calls; a user who fat-fingers `--max-attempts 10` could spend more than
   intended. Providers' billing is the backstop, but Praxis could do better.

4. **Summarizer is a single point of failure and a single model.**
   The summarizer uses one authed config (`configs[0]`). If that provider is
   down or rate-limited, the whole report degrades to empty findings (the run
   "succeeds" with no output via graceful degradation — which can look like a
   silent failure). There's no multi-model summarizer panel or fallback.

5. **Model-string drift is a perpetual footgun.**
   The registry ships model defaults (`deepseek-chat`, `glm-5.2`, `gpt-4o`,
   …) that *will* drift as providers release new models. We already hit this
   with Z.ai (`glm-4-plus` → `glm-5.2`) and with the gateway itself (the
   coding-plan URL). Users override via config, but the defaults rot, and a
   stale default fails with a provider-specific error that's confusing until
   diagnosed.

6. **No defense against prompt injection.**
   A document containing "ignore previous instructions…" is passed verbatim
   into the prompt. Praxis explicitly does not defend against this (it's an
   open problem in LLM tooling), but users critiquing adversarial documents
   may not realize their critique can be manipulated by the document itself.
   Documented in [Security Considerations](../book/src/security/considerations.md).

7. **Feature-gate ergonomics.**
   `cargo install praxis` installs default features only (`std`), which falls
   back to the echo backend with a notice. Real use needs
   `--features cli,backend-http,json`. This is documented but easy to miss;
   the "installed Praxis and it just echoes my doc" failure mode is real.

8. **No benchmarks or cost/latency characterization.**
   The docs claim sensible retry/timeout defaults but there's no data on
   typical run latency, token usage, or how it scales with panel size. Hard to
   plan production use without it.

9. **Limited real-world validation.**
   v0.1.0 is the first public release. The critique quality looks strong on
   the sample documents we've run, but there's no corpus, no comparison to
   single-model baselines, no measurement of whether the multi-critic approach
   actually catches more issues per dollar. The diversity claim is plausible
   but unproven at scale.

10. **Two-source-of-truth risk for docs.**
    Per the release process we chose the book as the single home for
    user-facing docs and *deliberately* skipped a `docs/guide/` mirror (which
    the IA release-polish convention otherwise suggests). This avoids drift
    now but means the repo's `docs/` directory contains only design docs and
    meta (ROADMAP, critique) — users expecting `docs/guide/` won't find it.

---

## What's Genuinely Good

- **Architecture.** The `Agent` trait boundary, the pure roster/panel
  resolution, and the sync/async bridge are clean and well-tested. Adding a
  new backend or topology is a contained change.
- **Agent-discoverability.** `capabilities` with dynamic auth state, `--dry-run`,
  structured error JSON, and documented exit codes make Praxis genuinely
  callable on the fly by AI agents — better than most CLIs.
- **Determinism where it matters.** Echo backend for tests; seeded roster for
  reproducible runs; graceful degradation so a run never hard-fails on a
  malformed LLM response.
- **Honest errors.** Provider attribution (`Devil's Advocate (glm-5.2) failed`)
  and exit codes make failures diagnosable, which is unusually good for a
  multi-provider tool.

---

## Verdict

Praxis v0.1.0 is a solid, well-architected foundation that delivers on its core
promise (diverse multi-critic critique with actionable findings) but has honest
gaps in security defaults (plaintext keys), cost guardrails, model-string
maintenance, and validation. The weaknesses are tractable and tracked; none is
architectural. For its first public release, it's in good shape.
