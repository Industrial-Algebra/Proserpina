# Design Documents

Each major feature of Proserpina has a design document in `docs/plans/`, capturing
the decisions made, the alternatives considered, and the open questions. These
are the canonical record of *why* Proserpina is shaped the way it is.

| Date | Document | Covers |
|---|---|---|
| 2026-06-19 | [`proserpina-design.md`](https://github.com/Industrial-Algebra/Proserpina/blob/main/docs/plans/2026-06-19-proserpina-design.md) | The original architecture: provider-agnostic graph engine, parallel/rounds/moderated topologies, the data model. |
| 2026-06-21 | [`multi-provider-roster-design.md`](https://github.com/Industrial-Algebra/Proserpina/blob/main/docs/plans/2026-06-21-multi-provider-roster-design.md) | The provider registry + seeded `random_roster` for diverse-model runs. |
| 2026-06-21 | [`credentials-config-design.md`](https://github.com/Industrial-Algebra/Proserpina/blob/main/docs/plans/2026-06-21-credentials-config-design.md) | Standalone TOML config for keys, model overrides, custom providers. |
| 2026-06-21 | [`rich-findings-design.md`](https://github.com/Industrial-Algebra/Proserpina/blob/main/docs/plans/2026-06-21-rich-findings-design.md) | The summarizer LLM pass + rich Finding model + dual markdown/JSON render. |
| 2026-06-21 | [`agent-readiness-design.md`](https://github.com/Industrial-Algebra/Proserpina/blob/main/docs/plans/2026-06-21-agent-readiness-design.md) | `capabilities`, `--dry-run`, structured errors + exit codes. |
| 2026-06-21 | [`panels-design.md`](https://github.com/Industrial-Algebra/Proserpina/blob/main/docs/plans/2026-06-21-panels-design.md) | Configurable persona panels (built-in + `[panels.NAME]`). |
| 2026-06-23 | [`retry-design.md`](https://github.com/Industrial-Algebra/Proserpina/blob/main/docs/plans/2026-06-23-retry-design.md) | Retry / timeout / backoff with config + CLI knobs. |

The CHANGELOG ties these to released versions.
