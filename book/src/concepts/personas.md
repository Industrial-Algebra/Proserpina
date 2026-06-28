# Critics, Personas, and Panels

A **persona** is the lens a critic applies: a name, an optional framing
sentence, and an optional focus area. It's *data*, not an enum — you can
configure critics without changing code.

## Built-in archetypes

| Persona | Framing | Focus |
|---|---|---|
| Devil's Advocate | Assume the proposal is wrong; find how. | logical gaps, unsupported assumptions |
| Methodologist | Scrutinize the rigor of every claim. | proof gaps, methodological soundness |
| Red Team | Find how this fails in practice. | failure modes, adversarial conditions |
| Domain Expert | Evaluate against the domain state of the art. | technical accuracy, novelty |
| Editor | Improve clarity and structure. | readability, missing context |

## Panels

A **panel** is a named set of personas. Built-ins:

| Panel | Critics |
|---|---|
| `default` | Devil's Advocate |
| `duo` | Devil's Advocate + Methodologist |
| `panel` | all five archetypes |

Select one with `--panel <name>`. Define your own under `[panels.NAME]` in the
[credentials file](../guide/panels.md).

## Provider assignment

Each critic in a panel is assigned a provider drawn (seeded, reproducibly)
from your authed set via the [roster](../guide/providers.md). A 5-critic
`--panel panel` run naturally spans multiple providers — the core of Praxis's
diversity value. Two critics may share a provider; persona- plus
model-diversity together is the goal, not provider uniqueness.
