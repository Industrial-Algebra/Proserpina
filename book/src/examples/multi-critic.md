# A Multi-Critic Run

This is a real `proserpina critique` output (DeepSeek + Z.ai glm-5.2, `--panel
panel`, seed 3) on a deliberately flawed proposal:

> *We will deploy a distributed consensus algorithm next month using eventual
> consistency, with no formal proof of safety and no conflict resolution
> strategy.*

## The command

```bash
proserpina critique proposal.md --panel panel --seed 3
```

## The dry-run plan

```json
{
  "seed": 3,
  "topology": "parallel",
  "roster": [
    {"persona": "Devil's Advocate",  "provider": "zai",      "model": "glm-5.2"},
    {"persona": "Methodologist",     "provider": "deepseek", "model": "deepseek-chat"},
    {"persona": "Red Team",          "provider": "zai",      "model": "glm-5.2"},
    {"persona": "Domain Expert",     "provider": "deepseek", "model": "deepseek-chat"},
    {"persona": "Editor",            "provider": "deepseek", "model": "deepseek-chat"}
  ],
  "n_critic_calls": 5,
  "n_summarizer_calls": 1,
  "estimated_total_calls": 6
}
```

Notice the roster fans the five critics across **both** providers — the
diversity value of the roster.

## The report (excerpt)

```
# Critique Report

**Subject:** `proposal.md`
**Findings:** 5 (3 blocker, 1 major, 1 minor, 0 info)

## 1. [blocker] The proposal attempts to combine consensus and eventual
         consistency, which are fundamentally incompatible.
- **Category:** logical contradiction
- **Quote:** > "consensus algorithm ... using eventual consistency"
- **Suggested change:** Choose a strong-consistency model (Raft/Paxos) or
  rename to a pattern that doesn't claim consensus.
- **Raised by:** Devil's Advocate, Methodologist, Red Team, Domain Expert

## 2. [blocker] No mechanism is provided for resolving concurrent writes,
         which prevents eventual convergence.
- **Category:** missing conflict resolution
- **Suggested change:** Add CRDTs, LWW, or a deterministic merge function.
- **Raised by:** Devil's Advocate, Methodologist, Red Team, Domain Expert
```

Two things to notice:

1. **The summarizer clustered across critics and providers.** Finding #1 was
   "raised by" four different critics spanning DeepSeek *and* Z.ai — the panel
   converged, and Proserpina tells you so.
2. **Each finding is actionable.** The `suggested_change` is concrete, not a
   vague "consider improving this."

The full run reproducibly produces these findings with `--seed 3`.
