# Bagua

Read the canonical [PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
at work start and again before delivery or Issue closure.

Bagua is a persistent terminal workspace. It absorbs tmux's mature models and
operational lessons, with explicit ownership of sessions, processes, terminal
state and client views. Its implementation forms an independent product.

Use Plectron's approach to capability selection, executable contracts and
substantial capability batches. Mature models can establish responsibility and
dependency boundaries early; experiments challenge the remaining choices.
Preserve declared consumer behavior rather than translating upstream files.
Keep supported subsets, deliberate exclusions and unverified behavior distinct.

Share mechanisms through their existing domain owners; terminal semantics and
workspace policy belong to Bagua. Linux x86_64 has a source-level persistent
local host, machine-control CLI and interactive clients; release and installed
qualification remain separate work. Executable capability output distinguishes models from available
behavior; neither a compiling interface nor an unavailable-operation refusal
proves terminal execution. Issue #6 establishes this source boundary; the
first Linux product closure is tracked by Issue #5.

## Ownership

The dependency direction runs from CLI and composition toward protocols,
models and engines. Direct consumers name the owning contract. Internal crates
are not independently published SDKs.

- `terminal` owns VT interpretation, screen state, modes, history, input
  encoding and snapshots. It has no workspace, client or OS process owner.
- `pty` owns native pseudoterminals, process association, bytes, resize, exit
  observation and reclamation. It does not interpret VT or own sessions.
- `workspace` owns session/window/pane identities, unique ownership, layout
  and state transitions. It performs no native IO or drawing.
- `protocol` owns requests, results, events, subscriptions and synchronization.
  It neither executes commands nor holds native resources.
- `client` owns local input, terminal capabilities, viewport, selection,
  copying and presentation. It does not own hosted processes.
- `lib` (`bagua`) composes the persistent host and its bounded execution.
- `cli` owns command grammar, startup and result presentation. Interactive and
  machine clients consume the same command semantics.

Session geometry and terminal screen extents are distinct owner types, mapped
at composition. Revisions belong to a target within a connection; a client
reconnecting to another host starts a fresh synchronization cursor. Missing
incremental history requires a complete snapshot. Stale or foreign-target
updates cannot replace installed state. These contracts do not freeze a wire
encoding before actual transport qualification.

World owns the authoritative object tree and ID allocation. Snapshots expose
read-only copies, not another writable authority. Window/pane ownership is
unique; IDs are not reused within one world. Closing a last pane removes its
window, while a session can remain empty. Exit retains the pane. Trusted native
outcomes use World observation, not a client command. A retired ID cannot be
recreated by a late outcome. Model retirement returns exact cleanup intents;
the host retains resource custody until actual native completion.

Joining never acquires control. Viewport reports from observers update only
their connection state. Acquisition and transfer validate the recipient's
viewport before changing automatic geometry. An explicit fixed acquisition
can recover control from a viewport too small for the existing layout. Layout
uses one separator cell, two minimum columns and one minimum row per pane;
ratios, limits and failure atomicity belong to the workspace model.

Operation and terminal input contracts are consumed through one protocol
request envelope. The pure evaluator reports applied model state or accepted
native intents. Neither is evidence for a completed native effect. Completion,
process and presentation events remain separate. Replica validation binds
target, baseline and the payload's internal revision before committing either
state or cursor. Latest is a single pending presentation slot; reliable events
and PTY bytes must never be given that droppable channel.
Subscriptions bind to the host's connection, require target-session membership
and retain at most 64 distinct exact targets. Cancellation remains meaningful
after target retirement. Resume revisions do not establish retained history;
the host must provide a full snapshot when a baseline is unavailable and
qualify actual subscription, transport and byte-budget enforcement.

## First product

A client disconnect or crash does not close the persistent host or its
application processes. Host exit, process exit, pane closure and session
destruction remain separate transitions. Preserving arbitrary processes and
recovering terminal state after host restart is outside the first contract.

Each session has one control client. Other connections observe without
implicitly acquiring control. Transfer is explicit; owner disconnect releases
control. Automatic geometry comes from the control client; observers have
independent viewports. Fixed geometry is explicit.

The host owns authoritative terminal state and the client draws it. PTY bytes
cannot be discarded to accommodate a slow client. View updates may coalesce
under explicit revisions; reliable control, process outcomes and presentation
remain different facts. Clipping and bounded work cannot depend on unchecked
unsigned subtraction or on a peer draining output promptly.

Linux is the first actual product endpoint. macOS and Windows enter the same
platform contracts, but each adapter needs its own native qualification.
ConPTY is not assumed to reproduce Unix signals or process-group semantics.
Codex is an ordinary consumer, with no agent loop or special core behavior.

## Selection

Bagua uses UTF-8 and a declared modern terminal profile. Actual shell and TUI
behavior determines necessary escape sequences, Unicode widths, modes, mouse,
paste and history; an old sequence is not obsolete merely because it is old.

The following compatibility obligations are excluded: tmux command syntax,
abbreviations and target expressions; scriptable `.tmux.conf`; format
expressions and shell-evaluated status; plugin and hook compatibility;
non-UTF-8 and ancient terminal specialization; broad historical Unix
portability; the complete implicit options inheritance system; and
server-side client-specific rendering.

The initial product also excludes shared windows across sessions, session
groups, simultaneous writers, a plugin VM and built-in remote network service.
Configuration describes data. Program, argv, working directory and environment
are explicit; invoking a shell is explicit when shell interpretation is needed.
These exclusions do not change how applications use their own shells.

## Verification

The first declared release contains only the Linux x86_64 GNU `bagua` executable.
Wharf owns its archive, manager, identity binding and complete distribution under
`releases.bagua.perish.uk`; internal crates remain unpublished. Source builds keep
version 0.0.0. Product source carries no generated installer or release workflow.
The required Windtun case names in `plumb.toml` are the consumer contract for
Seatrial #66. Declaration validity does not prove those scenes are published,
executed or qualified, or that stable promotion enforcement is active. Issue #12
retains the actual non-stable distribution obligation after its source merge.

Use focused tests while developing and the complete applicable Guard at
coherent delivery boundaries. Keep models and self-contained behavior here,
with thin executable smoke checks. Formally distributed-artifact multi-process
qualification belongs to the independent Seatrial scene, using only public
interfaces and isolated resources. Runtime source carries no testing mode.

The CLI exposes `capabilities`, explicit local machine control and structured
refusals. The terminal crate
implements the selected in-memory VT profile, snapshots and input encoding;
`PROFILE`, `EXCLUDED`, resource constants and executable tests own its exact
scope. Its dependency disables the growing OSC buffer. Do not enable that
feature transitively without requalifying the resource contract. The isolated
callback adapter preserves only upstream trait signatures; event and query
policy remains Bagua source outside that Ectropy boundary.

Full grapheme shaping, history reflow, DECAWM switching and extended keyboard
protocols are outside the current profile. Actual application needs may
challenge that selection before the first Linux closure. Preserve explicit
unsupported reports and refusals rather than reporting full VT coverage.
The terminal and PTY components are hosted on Linux x86_64, with explicit launch data, bounded
nonblocking IO and direct-child retirement/reaping. EOF, exit and closure are
independent facts. Retirement closes the master and signals the owned child;
it does not promise arbitrary descendant reclamation. The host must drive
observation after retirement and retain unwritten bytes across backpressure.
The host previews model mutations against its live and retiring resource budget.
Accepted input retains its unwritten suffix after disconnect. Completion follows
actual byte writes or native reclamation; model application is a separate fact.
Connections have increasing request IDs and explicit output admission. Only
presentation coalesces, with bounded rotating serialization. Reconnect starts a
fresh actor/cursor and does not authorize replay. Inspect returns a complete
screen and process facts before finishing. Current runtime screens have zero
scrollback; larger terminal-model history is not a runtime capability claim.
The socket lives in an explicitly supplied canonical private user directory;
peer UID comes from the kernel. Never replace an unknown existing endpoint.
The interactive client consumes typed full snapshots over the same local protocol.
Its replica validates target, dimensions, cells and revision before installing.
Pane retirement never resurrects a display. Selection freezes a local copy;
it never writes host state. The local copy buffer is not an OS clipboard claim.
Client output is generated from typed cells/styles and clips with checked bounds.
Raw PTY data and host escape strings never pass through to the outer terminal.
Keep input, pending requests and output bounded independently; only presentation
coalesces; adjacent unsent text may batch without changing byte order. Normal/error cleanup restores
terminal modes; a client death cannot retire hosted processes. Ordinary modern
UTF-8 VT/SGR-mouse behavior is the selected outer-terminal profile. It does not
qualify every emulator, extended key protocol or unsupported shaping behavior.
Other native platforms require their own evidence.
Keep source composition/smoke evidence distinct from formal distributed-artifact
host/client scenes and installed product qualification.
