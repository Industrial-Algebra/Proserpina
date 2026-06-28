# Praxis — v0.1.0 Release Prep

- **Date:** 2026-06-23
- **Status:** In progress
- **Branch:** `feature/release-prep-v0.1.0`
- **Goal:** Full IA-convention release — public repo, crates.io publish,
  mdbook docs, CI workflows.

## 1. Scope (decided)

Per Justin: **public repo + crates.io + book + netlify + publish/docs CI.**
Maximizes the deploy-everywhere / agent-callable value (`cargo install praxis`,
public docs agents can read). `praxis` crate name confirmed available on
crates.io.

## 2. Deliverables

### A. README rewrite (High)
Replace the scaffold README with a real one matching Schubert's structure:
- Tagline + one-paragraph "what is Praxis"
- **Why multi-agent critique?** (the diversity argument)
- **How it works** (graph engine, roster, summarizer — high level)
- **Quick start** (`cargo install praxis`, set a key, `praxis critique doc.md`)
- **Configuration** (credentials.toml: providers, panels, retry)
- **Panels** (built-in default/duo/panel, custom `[panels.NAME]`)
- **Agent integration** (`praxis capabilities`, `--dry-run`, `--json`,
  exit codes)
- **Examples** (a sample run output)
- **Features** table
- License / commercial license section

### B. CHANGELOG.md (High)
`## [0.1.0] — 2026-06-23` with `### Added` sections grouped: Core engine,
Backends, Multi-provider, Reports, Agent integration, Reliability. Summarizes
the 13 PRs as user-facing features.

### C. CI workflow `.github/workflows/ci.yml` (High)
Mirrors Schubert's: fmt / clippy (`--all-targets --all-features -- -D warnings`)
/ test (default + all-features) / doc, on push/PR to develop+main. Praxis has
no sibling-crate checkouts (unlike Schubert's amari/karpal deps, which are
crates.io-published), so it's simpler.

### D. CONTRIBUTING.md (Medium)
IA CLA boilerplate + Praxis-specific: IA gitflow (feature→develop→main),
TDD expectation, the four CI gates, clippy `-D warnings`, AGPL headers.

### E. mdbook `book/` + `book.toml` + `netlify.toml` (Medium)
IA Navy theme (per the ia-mdbook skill). Structure: Introduction, Getting
Started, Configuration, Panels, Agent Integration, Architecture, Design docs
index. Netlify deploy config.

### F. Publish + docs workflows (Medium, post-public-flip)
- `.github/workflows/publish.yml` — on tag `v*`, `cargo publish` to crates.io
  (needs a `CARGO_REGISTRY_TOKEN` secret).
- `.github/workflows/docs.yml` — build the book, deploy to netlify on main.

### G. Examples expansion (Low)
Add 2–3 examples beyond `deepseek_smoke.rs`: a multi-critic `panel_smoke.rs`,
a credentials-config example, maybe a library-usage example.

### H. Repo public flip + crates.io publish (the release action itself)
- `gh repo edit Industrial-Algebra/Praxis --visibility public`
- Tag `v0.1.0` on main, push → publish workflow fires.
- Verify `cargo install praxis` works.

## 3. Sequencing

Do the *safe, content* work first (commits to the branch), defer the
irreversible public/publish actions to the end with explicit Justin approval:

1. README rewrite.
2. CHANGELOG.md.
3. CONTRIBUTING.md.
4. CI workflow (ci.yml).
5. mdbook + netlify config.
6. Examples expansion.
7. publish.yml + docs.yml workflows.
8. Verify all gates + book builds locally.
9. **[Justin approval gate]** PR → develop → main, tag v0.1.0.
10. **[Justin approval gate]** repo public + crates.io publish.

Steps 9–10 are irreversible/external and wait for explicit go.

## 4. Out of scope

- Code changes (this is polish + release mechanics only; the codebase is
  feature-complete from PR #13).
- The deferred backlog (circuit breaker, moderated topology, etc.) — post-0.1.0.
