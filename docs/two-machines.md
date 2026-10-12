<!-- SPDX-License-Identifier: MIT -->

# Two or more machines

Run your agents on one machine (or several) and watch and use them from another. You need a network route from each machine to the machine that runs the orchestrator, TCP port 7676 by default. Nothing else: no SSH, no open port on the machines that run agents.

Each machine needs Flight installed ([install.md](install.md)). In this example:

- **Machine A** runs the orchestrator and shows the dashboard.
- **Machine B** runs your agents (a node).

You can add more nodes the same way, and A can be a node too.

Commands marked `A` or `B` are typed on that machine. Commands that print a **bundle** print one line of text; copy the whole line.

## 1. Start the orchestrator (A)

Find A's address as B will reach it, for example `ipconfig getifaddr en0` (macOS) or `hostname -I` (Linux). Use it here as `A_ADDRESS`:

    A$ flight orchestrator run --listen 0.0.0.0:7676 --advertise A_ADDRESS:7676

Leave it running. It prints `listening on …` and `advertising A_ADDRESS:7676`. macOS may ask whether `flight` may accept incoming connections: allow it. (To keep it running without a terminal, see [services.md](services.md).)

## 2. Add the node (B)

In a second terminal on A, make a one-time bundle:

    A$ flight orchestrator enrollment create

It prints one line starting `orchestrator=sha256:…`. On B, put the whole line in single quotes:

    B$ flight node join 'orchestrator=sha256:… address=A_ADDRESS:7676 token=… expires=…' --name machine-b
    B$ flight node run

`join` prints `joined orchestrator …`. `node run` prints `connected to A_ADDRESS:7676`; leave it running. A bundle works once and expires after a few minutes. If `join` says `enrollment refused`, make a new one.

Check on A:

    A$ flight orchestrator node list
    node sha256:… Online panes=0 machine-b

## 3. Add the dashboard (A)

    A$ flight orchestrator enrollment create
    A$ flight ui join '<that bundle>' --name laptop
    A$ flight ui

Press `n`: `machine-b` is listed as a place to start a workspace. Choose it and a folder **on B**, and the agent starts on B and shows on A.

## 4. Try it

- Create a workspace on `machine-b`, open the shell (`s`), run `echo hello > marker`, press `Ctrl-Space` then `q`, quit the dashboard (`q`) and start `flight ui` again: the workspace and the file are there.
- Turn B's network off for a minute and on again: A shows B as stale, then disconnected, then online again by itself. The agent kept running throughout.
- Stop `flight node run` on B (Ctrl-C) and start it again: the same.

## Remove a machine

    A$ flight orchestrator trust list
    A$ flight orchestrator trust revoke <id>

The machine is cut off at once. Its sessions keep running on it, and you can end them there.

## Keep it running

Run `flight node run` (and `flight orchestrator run`) from the system's service manager, so they start by themselves and restart if they fail: [services.md](services.md).

## Good to know

- The orchestrator can live on one of your machines, on a server, or hosted: nodes dial it, and the dashboard cannot tell which. It is meant for a network you trust; exposing it to the public internet is not supported in 1.0.
- A node on another machine needs the agent program (`claude`) installed there. If it is missing, the dashboard says `claude was not found on this node's PATH`.
