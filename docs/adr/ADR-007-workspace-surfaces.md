<!-- SPDX-License-Identifier: MIT -->

# ADR-007: Flight Workspace and its surfaces

Status: Implemented for two surface kinds, Agent and Shell; a workspace may hold several surfaces of one kind (see "Several surfaces of a kind"). Scope: one host and one root directory per workspace. Not in scope: Advisor or any other surface kind, arbitrary programs, plugins, multi-host workspaces, a workspace database, project discovery, repository cloning, templates, resuming an agent.

## Decision

The user-facing unit of Flight is the **Workspace**: the user's project/work context, on one host, rooted in one directory. A workspace has **Surfaces**, typed resources attached to it. The first two kinds are **Agent** and **Shell**.

> A Flight Workspace represents the user's project/work context. Surfaces are resources attached to that workspace. Claude, Codex, a shell, or any future tool does not define the workspace itself.

So the user thinks "open my nga workspace", not "open pane %12 on server work".

> Providers such as Claude or Codex are implementations of Agent surfaces. They do not define the Workspace.

> tmux is currently a backend used to realize persistent terminal surfaces. It is not part of the Flight user model.

## Model

```
Workspace                      Surface
  id: WorkspaceId                id: SurfaceId
  name                           workspace: WorkspaceId
  host                           kind: Agent(AgentKind) | Shell
  root (a directory on the host) state: the existing AgentState
  surfaces
```

- `WorkspaceId` and `SurfaceId` are distinct typed ids (`flight-state`). A display name never stands in for either, and neither is a tmux id. Minted by the node (64 random bits, `w-…`, `s-…`); 1 to 64 bytes of `[A-Za-z0-9_.-]`, validated at the edge like every other wire string.
- A workspace is **derived, never stored**. There is no workspace table for *live* state (a durable record of intent is ADR-008): the dashboard groups the panes the nodes publish by workspace id. Flight still has no durable central live state.
- Attention is derived from the Agent surface only. The workspace's summary state, its sort order and its "needs you" are its agent's. Each surface keeps its own state (`Agent: Question`, `Shell: ready`); a shell never asks for the user. A workspace with no agent is as urgent as a shell.
- A workspace may hold several surfaces of the same kind, each with its own `SurfaceId` (see "Several surfaces of a kind"). Without saying so, asking for a second shell is still refused (`AlreadyExists`), and the dashboard simply opens the one that exists.

## Several surfaces of a kind

Decided after v1, which had at most one surface of each kind: a workspace may contain any number of surfaces of the same kind. What identifies a surface is its `SurfaceId`, never its kind or its position; the kind says what it is, not which one.

- **Default names are unchanged.** In a layout the first agent is `agent` and the first shell is `shell` (a name that keeps meaning "the workspace's shell" if its window is replaced). "First" is the oldest: surfaces are ordered by the backend's own creation order (a tmux pane number), not by their random ids, so adding a shell never changes which one `shell` means. Every other surface is named by its own id in layouts, tabs and saved arrangements, for as long as it exists. A saved layout that names a surface that is gone drops it.
- **Explicit creation only.** `CreateSurface` gains `another` (field 3, capability `multiple_surfaces_v1`). Without it the rule is the old one: a workspace that has a shell refuses (`AlreadyExists`), so two UIs racing to make the first shell still make one. With it a new shell is added beside the others, in the workspace's directory, with a new id. Nothing creates one on its own: not opening a workspace, not a restore of a workspace that had one (a restore brings back exactly the shells that were saved), not a recovery. The dashboard asks first (`S`: "Add another shell to ...?") and opens the one it made.
- **Removal is explicit and alone.** `RemoveSurface { workspace_id, surface_id }` (`multiple_surfaces_v1`) kills that surface's window and nothing else; the node finds it by the id its window is marked with, inside the workspace's own session. Only a shell is removed (an agent goes with its workspace), never a window the workspace has no mark for, and never a session's last window (that would end the workspace). A saved workspace forgets the surface, so a later restore does not bring it back. In a presentation it is `Ctrl-Space X`, asked twice (anything between cancels), and only with another surface left to show.
- **Admission uses the terminal limits.** Surfaces are windows of the workspace's session, not terminals; what a surface costs is the terminal opened onto it, and the existing limits (per interface, per node, in all; ADR-009) are what admit or refuse that, whatever kind it is. There is no per-kind or per-workspace count.
- **Names in the backend.** Windows of one kind get distinct names (`shell`, `shell-2`, ...) because the backend addresses a window by name while it is being marked; the mark (`@flight_surface`) stays the kind.
- **Not a terminal by assumption.** Nothing above relies on a surface being a terminal. The workspace model (ids, kinds, creation, removal, persistence, naming in layouts) is stated in terms of surfaces; a pane, a PTY and tmux appear only where a terminal is opened onto a surface, and a new kind is still a new `SurfaceKindCode` and a new `@flight_surface` value. Creation and removal of a kind are decided by that kind, not by the generic path: only the shell is removable and creatable today.
- **Compatibility.** A node or orchestrator without `multiple_surfaces_v1` never receives `another` or a removal (the orchestrator refuses them as unsupported, the dashboard says the node is too old), and a request without `another` means exactly what it did. Saved workspaces already list surfaces by key, so a workspace with several shells needs no migration; one saved before this keeps its one shell.

## Where the metadata lives, and surviving restarts

In the backend that already keeps the sessions alive, as tmux user options, so it survives a node restart, an orchestrator restart and a UI restart with no database:

| Option | Scope | Meaning |
|---|---|---|
| `@flight_session` | session | existing: the node publishes every pane of this session |
| `@flight_workspace` | session | the `WorkspaceId` |
| `@flight_surface` | window | `agent` or `shell` |
| `@flight_surface_id` | window | the `SurfaceId` |

The workspace root is tmux's own `session_path`, the directory the session was started in. A workspace is one session; each surface is a window of it. The node reads these in the same `list-panes` it already runs (`PANE_FORMAT` gains five fields, before the title, which stays last) and publishes them on the `PaneState` it already sends, so snapshots, deltas, resync and reconnect rebuild workspaces and surfaces for free. The options are written by the same tmux invocation that creates the session or window, so no observer can see one unmarked.

## Existing sessions

A session without these options is not refused and not migrated. The node synthesizes a minimal workspace around it: the id is derived from the host and the backend's own session identity (`legacy.<host>.<server>.<n>`), never from the name, so renaming the session keeps it; the pane is an Agent surface if it runs a known agent and a Shell surface if it is only published because its session is Flight's; the root is the session's start directory. The identity lasts as long as the backend does, like a pane id. A node that predates workspaces publishes none of the new fields, and the dashboard treats each pane as a workspace of its own. The rules live once, in `flight-state` (`RawPlacement`), and are used by the node and by the direct collector alike.

## Protocol

Flight semantics, not tmux semantics:

- `PaneState` gains `workspace_id` (15), `surface_id` (16), `surface_kind` (17, `SurfaceKindCode { Agent, Shell }`) and `workspace_root` (18). Absent from an older node; present, they are validated.
- `Command.kind` tag 9 `CreateSurface { workspace_id, kind }`, capability `create_surface_v1`. It carries no host and no directory: the orchestrator finds the host that publishes the workspace (an id nobody publishes, or that two hosts claim, is `UnknownWorkspace`, never routed on a guess), and the node resolves the session and the directory from the workspace itself, reading its backend live. A caller cannot point a surface at another host or directory because there is nowhere to say so. A disconnected node is `NodeUnreachable` at once; nothing is queued.
- `ErrorKindCode.UnknownWorkspace = 15`.
- `CreateSession` is unchanged on the wire and now means "create a workspace": the node mints the ids and marks the new session. The `Shell` program remains for other callers but the dashboard no longer offers it: a workspace is made for an agent and gets its shell afterwards.
- `Agent` is refused as a `CreateSurface` kind: an agent is created with its workspace. There is no `Advisor` value; a new kind is a new number, and a peer that does not know it refuses the message rather than guessing.

## Shell surface

Added to an existing workspace by `CreateSurface` (the first one; others with `another`). The node starts it in the workspace root with the node's default shell, started the way tmux starts one for a session made by hand (no shell is hardcoded, no command line is accepted). It is one window of the workspace's session, so it persists, can be re-attached and survives leaving the UI. If the shell cannot be marked, the one new window is removed (by its own id, and only if unmarked); if it exits at once, nothing is reported created. Requests are serialized per node so two cannot both pass the "no shell yet" check (an `another` request does not make that check).

## Presentation

Every surface is shown the same way: through Flight's own terminal path (a real tmux client on a pty the node owns, bytes relayed untouched). Flight does not emulate a terminal.

- Dashboard: one row per workspace (name, agent state, host last); the selected one opens out to its surfaces, `a Agent` and `s Shell` (the oldest shell), each with its own state; `S` adds another shell after asking. A missing shell says "none yet · press s to create".
- `Enter` or `a` opens the agent, `s` the shell. `s` on a workspace with no shell opens "Companion shell": the workspace's directory and host are shown, not asked for; Create (Enter or `y`) makes it and opens it as soon as it is published. This replaces going through New and re-entering a host and a path.
- Inside a session: `Ctrl-Space a` and `Ctrl-Space s` switch to the other surface of the same workspace without the user touching the list (the dashboard returns for a moment only to open it), `Ctrl-Space q` returns to the dashboard, `Ctrl-Space Ctrl-Space` sends a literal Ctrl-Space. Every other key is the program's own.
- New session became New workspace: host, name, directory, agent (Claude, or Claude without permission prompts). It is created with its agent; the shell is one `s` away.

## Side by side: not done

Showing the agent and the shell at once, both interactive, needs one terminal client per surface composed on a single screen. Flight forwards bytes and understands none of them, so composing two live terminals means emulating one (a screen model per surface and a compositor), or asking tmux to split a window, which makes the surfaces panes of one tmux layout and gives up their independent presentation. Neither is a small change to the presentation path, and a stale `capture-pane` rendering would not be interactive, so it is not offered. It is the next UX step: a terminal-emulation layer in the client, behind the same surface identities. Fast switching is what ships.

## Consequences

- The dashboard's selection is the workspace's identity, so it follows the workspace when its agent window ends and only the shell is left.
- tmux terms still appear in the backend and the protocol-internal names (`PaneRef` in the existing commands, pane ids in logs); no screen the user reads uses them (checked by a test that renders the dashboard, the offer, help and the form).
- A workspace's agent is whatever the Agent surface runs; adding Codex as a provider needs no change to this model.
- Future surface kinds (an advisor chat, logs, a test runner) are new `SurfaceKindCode` values and new `@flight_surface` values, attached to the same workspace identity.
