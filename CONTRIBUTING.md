# Contributing to Proserpina

Thank you for your interest in contributing! Proserpina is an Industrial Algebra
project, dual-licensed under AGPL v3 and a commercial license.

## Contributor License Agreement (CLA)

Proserpina is dual-licensed (AGPL v3 + commercial). To enable this model, **all
contributors must sign a Contributor License Agreement (CLA)**.

The CLA grants Industrial Algebra the right to relicense your contributions
under the commercial license, while you retain full copyright ownership of your
contributions. Without a CLA, your contributions can only be used under AGPL
v3 terms, which would prevent Industrial Algebra from offering a commercial
license for the combined work.

### How to Sign

1. Download the CLA from: https://industrial-algebra.org/cla
2. Sign and email to: cla@industrial-algebra.org
3. Include your GitHub username in the email

Pull requests from contributors who have not signed the CLA cannot be merged.

## Development Setup

```bash
git clone https://github.com/Industrial-Algebra/Proserpina
cd Proserpina
cargo build --all-features
cargo test --all-features
```

The default build (no features) is sync, key-free, and network-free — it pulls
in zero HTTP/async/rand/toml deps. HTTP, the roster, and the summarizer are all
behind the `backend-http` feature.

## Conventions

- **Rust edition 2021**, nightly toolchain (`rust-toolchain.toml`).
- **Test-driven.** Write the failing test first, watch it fail, then implement.
  No implementation code without a failing test.
- **Never panic in library code.** Every fallible public operation returns
  `Result<_, ProserpinaError>`.
- **Additive feature gates only.** Features add capability; they never remove
  existing API. Document every feature in `src/lib.rs`.
- **Every public item is documented**, with `# Examples` / `# Errors` sections.
- **AGPL headers** (`// Copyright (C) 2026 Industrial Algebra` +
  `// SPDX-License-Identifier: AGPL-3.0-only`) on every `.rs` file.

## The Four CI Gates

All four must pass, on **both** default and `--all-features`:

```bash
cargo fmt --check
cargo clippy --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
```

## Git Workflow — IA Gitflow

```
feature/* ──PR──► develop ──release PR──► main
```

- Branch from `develop`; open PRs against `develop`.
- Human review only — no auto-merge.
- Releases: release PR `develop` → `main`, tag `vX.Y.Z` on `main`.

## Pull Request Process

1. Sign the CLA (above).
2. Implement with TDD; ensure all four gates pass on both feature sets.
3. Document new public items; update the CHANGELOG under `[Unreleased]`.
4. Open the PR against `develop` with a clear description + verification.

## License

By contributing, you agree that your contributions will be licensed under the
same dual-licensing model as the project (AGPL v3 + commercial).
