<!-- SPDX-License-Identifier: MIT -->

# ADR-013: Agent providers

Status: accepted for v1.1.0 (#120). Scope: choosing which agent a workspace runs, without Flight's workspace model depending on one agent. Claude Code is the verified baseline. Codex is added behind the same interface and declared supported only when it passes the same integration tests (below). Not in scope: other agents, discovering sessions Flight did not start (ADR-010), changing state detection (`flight-classify` already recognises Codex), new surface kinds.

## Where Claude is hard-wired today

- `Program` (UI and node) is `Claude | ClaudeSkipPermissions | Shell`; the wire's program code is the same closed set.
- `session_create::create` finds `claude`, builds its argv through `Claude::new_argv`/`resume_argv`, and words its errors for Claude.
- `resume_support(provider)` is a table, with Codex marked unsupported (ADR-010); `SurfaceSpec.provider` is already a string in the saved definition.

So the saved model is provider-neutral; the **launch path and the request that reaches it are not**.

## Decisions

1. **A provider is an adapter behind a small interface** in `flight-node`: its id (`claude`, `codex`), the program to find on the node's `PATH`, how to build the command line for a new session and for a continuation, how it spells "do not ask before acting" (or that it has no such mode), whether and how its sessions can be continued (ADR-010's `Support`), and the words for its errors. `create` goes through it; nothing outside the adapters names a program.
2. **The wire carries a provider id, additively.** The create request gains an optional `provider` string (and keeps the program code for older peers, which map to `claude`). A node advertises the providers it can launch in its capabilities; the dashboard offers only what the chosen host advertises, so an older node is never asked for Codex. Peers that do not know the field ignore it (the same rule as every additive field since 1.0); the capability name is `agent_providers_v1`.
3. **"No permission prompts" stays a spelled-out, per-provider choice.** The form shows it as a separate option with the provider's own wording, never as a default, and it keeps the existing recovery rule (such an agent is not started or resumed unprompted).
4. **The choice is the person's.** The form lists the providers the host offers, in the order of the person's preference from `settings.toml` (ADR-012), with the preferred one selected. With no preference, Claude.
5. **Continuation is a property of the provider, stated honestly.** Claude: supported (ADR-010). Codex: `Unsupported` stays (it assigns its own ids and reports none); a saved Codex workspace restored after tmux is lost **starts a new Codex conversation and says so**, and never claims to have continued one. `codex resume --last` is not used: it picks the most recent session of the directory, which may not be Flight's (guessing, which ADR-010 rules out).

## When a provider is "supported"

A provider is listed as supported in the documentation and without a qualifier in the form only when it passes the **equivalence suite**: the same scenarios the Claude path passes, run against that provider's real or faithfully faked CLI:

| # | Scenario | Claude | Codex requirement |
|---|---|---|---|
| 1 | Create a workspace on a node: the agent starts in the verified directory; an agent that is not installed is refused with the install hint | tests exist | same |
| 2 | The agent surface is reported with its provider and state (state from `flight-classify`) | tests exist | same |
| 3 | Companion shells, naming, quit and return: the session survives the dashboard, the node, the orchestrator | `journey.sh` | same with the provider |
| 4 | Tmux server lost: the workspace is saved; restore starts the agent in the same directory | `journey.sh`, `persistence_live.rs` | same; **new conversation, stated**, never "continued" |
| 5 | Permission-free mode needs explicit choice, is never started unprompted by recovery | tests exist | same, with the provider's own flag |
| 6 | The real program, scripted end to end | `real-claude.sh` | `real-codex.sh` (needs a logged-in `codex`) |
| 7 | Mixed fleet: an older node never receives a provider it did not advertise | new | new |

Until 1-7 pass, Codex is not offered by default (it can be enabled in `settings.toml` and is labelled as not yet supported). The readiness matrix records the evidence.

## Consequences

- The workspace architecture keeps one notion of an agent surface with a provider id; adding a provider is one adapter and its suite, not edits across the node, wire and form.
- Wire changes are additive and capability-gated; 1.x peers interoperate.
- Codex users get launch, state and persistence; continuation of conversations is not promised.
