<!-- SPDX-License-Identifier: MIT -->

# How Flight works

You do not need this to use Flight. Read it when you want to know what survives what.

## The three parts

- The **node** runs on each machine that has your agents and shells. It starts them, watches them and reports.
- The **orchestrator** is the meeting point: nodes and dashboards connect to it. On one machine, `flight` starts it for you.
- The **dashboard** is what you look at and type into. Any machine can show it.

On one machine, plain `flight` starts all three in the background for as long as the dashboard is open. With more machines, you start them yourself ([two-machines.md](two-machines.md)); nodes and the orchestrator are best run as services ([services.md](services.md)).

## Sessions live in a private tmux

Your agents and shells run inside a **private tmux server** (named `flight`), which Flight starts and which does not touch your own tmux. tmux is what keeps them alive when the dashboard closes. You never need to use it.

## What survives what

These are three different things.

1. **The session survives** (nothing needs doing). Closing the dashboard or terminal, restarting or killing the node or the orchestrator, losing the network: the agent and shells keep running in the private tmux. When the dashboard or the node comes back, everything is where you left it, including what is on the screen.
2. **A saved workspace is restored** (you press Enter). If the private tmux server is lost (the machine restarted, or someone ran `tmux -L flight kill-server`), the processes are gone, but Flight remembered each workspace: its machine, folder, agent and shells. It shows as **saved**. Press Enter and it starts again in the same folder, never a different one. The shells are new; what was running in them is not recovered.
3. **The conversation is resumed** (Flight asks the agent to, when the agent allows it). Claude does; Codex does not (it chooses its own conversation ids and does not report them, so Flight cannot tell which one is yours): a saved Codex workspace is restored as a **new** Codex conversation, and the dashboard says so before you press Enter. When a saved Claude workspace is restored, Flight starts Claude with the conversation it remembered (`claude --resume`). If Claude still has it, the earlier conversation continues. If not, Flight says so and offers a new conversation; it never pretends.

## Where Flight keeps things

Under `~/.config/flight` (one folder per role: `orchestrator/`, `node/`, `ui/`): identities, trust, and your saved workspaces (`node/workspaces.toml`). Files that hold secrets are private to you. A newer file is never read by an older Flight, and files are never overwritten by one that cannot read them.

## Trust between machines

Machines trust each other by a one-time **enrollment bundle**: the orchestrator makes one, you paste it on the new machine, and from then on the connection is mutually authenticated (TLS 1.3, each side pinning the other's key). A bundle works once and expires. `flight orchestrator trust revoke` drops a machine at once.
