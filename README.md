# Bagua

A persistent terminal workspace, built in the PerishLab domain.

Bagua is a wholehearted tribute to [tmux](https://github.com/tmux/tmux), with a few
engineering preferences of my own. Its mature design and practical lessons are
the foundation for this exploration.

Bagua explores explicit ownership of sessions, processes, terminal state and
client views. It follows executable contracts and bounded capability delivery.

The seven-crate workspace implements model contracts for control ownership,
geometry, process lifecycle and revision synchronization. The terminal model
also interprets a declared UTF-8/VT subset and encodes mode-dependent input.
The CLI provides capability inspection and explicit refusal of unavailable
operations without a host connection. Linux x86_64 has a persistent local host
and machine-control CLI, plus interactive clients. Formal artifact qualification
and installation remain separate delivery work.

## Workspace and control

`workspace::World` owns one session/window/pane tree. IDs are nonzero and never
reused within that world's lifetime. Clients join as observers. Acquiring or
transferring control is explicit and checks the connected client and layout
before committing. Observers can report their own viewport without changing
session geometry. Automatic sizing follows the controller's accepted viewport;
fixed sizing preserves an explicit canvas. A narrow observer can acquire with
an explicit fixed size when automatic sizing cannot fit the current layout.

Column and row splits use a ratio of 1–999 thousandths and one separator cell.
Panes need at least two columns and one row. A refused model split, handoff or resize
preserves the previous tree, focus, policy, revision and ID allocator. Closing
a pane collapses its split; an empty window is removed, while an empty session
remains until explicitly closed. Focus falls back to the lowest surviving ID.
New windows and splits select their newly created object. Process exit retains
the pane and its outcome. Disconnect releases control without closing objects.

The model limits one world to 32 sessions and 4096 panes, one session to
32 windows and 64 attached clients, and one window to 64 panes and 16 tree
levels. Catalog snapshots own immutable bounded model state. Workspace geometry
uses nonzero 16-bit dimensions; the host must additionally enforce the terminal
engine's narrower profile before creating actual terminal resources.

CLI grammar, interactive request constructors and programmatic callers share
`protocol::Request`. Request numbers are nonzero; the default constructor uses
1 for a single request, while a persistent caller supplies its own numbers.
Connection identity is supplied by the host, never accepted from a command.
IDs and object kinds are exact; human names and tmux target syntax are absent.

The pure `bagua::evaluate` composition applies model changes. Input and pane
retirement return accepted intents; they perform no native IO and do not
produce completion evidence. Native completion/failure, process outcomes and
presentation revisions have separate event types. Late outcomes for retired
IDs cannot recreate objects. The CLI requires an explicit socket for hosted
operations. Local capability inspection describes compiled endpoints, not whether
a host is running. Native effects report their actual completion separately.

Synchronization binds target, baseline and payload revision. A replica installs
both state and revision only after validation and successful delta merging;
gaps require a full snapshot. `Latest` keeps one pending versioned presentation
per subscription and coalesces it into a complete snapshot. Reliable events
and PTY bytes do not implement that contract. Payload size belongs to its model
owner, and subscribers own their retention. Host restart creates a fresh
connection/cursor lifetime. Subscribe/unsubscribe are explicit intents; the
connection-local metadata set admits at most 64 exact targets and treats a
duplicate subscription idempotently. Observers must be joined to the target's
session. Cancelling a retired target remains possible. A resume revision is a
hint, not permission to invent a delta without its baseline. The host sends
complete snapshots and retains no incremental history. Wire version 2 uses a
four-byte big-endian JSON length followed by that many UTF-8 JSON bytes.
Request numbers strictly increase within a connection; reused or older numbers
are refused without replay. There is no cross-connection exactly-once claim.
Unknown versions, commands and invalid identities are refused.

```sh
cargo run --locked -p bagua-cli -- capabilities --json
```

The binary uses the published PerishLab Plumb substrate. Building requires
access to the `perish` Cargo registry; registry declaration is included, while
credentials are supplied by the developer's environment. Unauthenticated
registry access has not been qualified. No private SDK source or credentials
are copied into this repository.

## Native PTY

`pty::Process` owns a Linux x86_64 PTY master and its direct child. It requires
Linux 4.13 or later with a working `/dev/pts` mount. Other targets return
`Unsupported`; macOS and Windows have no qualified adapter yet.

Launch specifies an absolute executable, literal argv, absolute working
directory, complete environment and nonzero dimensions. The environment uses
the published `radicula-process` contract and inherits no variables. There is
no implicit shell or working-directory fallback. Arguments and environment
each have at most 256 entries, with a combined 65,536-byte launch budget.
Rustix supplies syscall wrappers; no upstream implementation is copied.

The child starts a new session with its slave as controlling terminal and
stdin/stdout/stderr. Reads and writes are nonblocking and transfer at most
4096 bytes per call. `Pending` preserves caller custody of unwritten bytes;
callers drain output without dropping bytes. Poll waits are capped at one
second. Hangup readiness requires draining reads; PTY EOF, child exit and
resource closure remain separate observations. Resize updates the kernel PTY
and delivers its native foreground-group notification.

`retire` closes the master and requests SIGHUP of the owned direct child.
Kernel terminal hangup also affects the terminal's foreground process group.
Driving `observe` escalates a surviving direct child to SIGKILL after a
250 ms grace period and reaps its actual exit status. `close` drives this to
completion synchronously; `Drop` provides the same cleanup fallback. Neither
is a hard wall-clock deadline. A host should drive retirement through its
event loop. Client disconnect does not imply retirement. Detached descendants
and arbitrary process trees are outside this cleanup guarantee.

Native tests cover controlling-terminal setup, explicit launch data, binary
IO, output and input backpressure, SIGWINCH resize, exit status versus EOF,
forced cleanup, launch failure, descriptor custody and cancellation. These
qualify the component on the tested Linux machine; persistent hosting and
distributed-artifact application scenes require their own evidence.

## Local host and machine control

Use an existing canonical absolute directory owned by the current user, with
no group/other permission bits. The host creates a mode-0600 Unix socket there
and validates each peer's kernel UID. Existing sockets or files are never
replaced automatically. Graceful shutdown removes only the socket inode this
host created. After an abnormal host exit, inspect any stale endpoint before
removing it. The host has no implicit startup, TCP endpoint or remote identity
service. `serve` runs in the foreground; SIGINT, SIGTERM and explicit `shutdown`
drive owned-process retirement. There is no host-restart recovery contract.

With the binary built, choose a real private directory in place of the example
path below. Run `serve` in one terminal; the remaining commands use that host.
On a fresh host the first session/window/pane IDs are 1/2/3; consume returned
IDs rather than assuming these numbers in an existing host.

```sh
bagua --socket /absolute/private/bagua/host.sock serve
bagua --socket /absolute/private/bagua/host.sock new --json
bagua --socket /absolute/private/bagua/host.sock --control 1 start 3 \
  --executable /bin/sh --cwd / --arg=-i \
  --env PATH=/usr/bin:/bin --env TERM=xterm-256color
bagua --socket /absolute/private/bagua/host.sock --control 1 send 3 'printf bagua-ready'
bagua --socket /absolute/private/bagua/host.sock --control 1 key 3 enter
bagua --socket /absolute/private/bagua/host.sock --session 1 inspect 3 --json
bagua --socket /absolute/private/bagua/host.sock --control 1 close session 1 --json
bagua --socket /absolute/private/bagua/host.sock shutdown
```

`new`, `window` and `split` create pending panes; only explicit `start` launches
a program. `--session ID` joins as an observer for a command. `--control ID`
explicitly joins and acquires control using the current canvas as its viewport;
it refuses if another connection controls the session. These setup operations
consume request numbers beginning at `--request`. Neither option transfers
custody of hosted processes to the CLI. Command exit releases its connection
and control. `attach` is currently a machine join operation; the interactive
interface is available through `open` and `connect` below.

`control` accepts one JSON `protocol::Request` per stdin line, on one persistent
connection. It writes JSON messages to stdout and waits for each request's
actual result before reading the next line. `watch` streams subscribed updates
on its own connection. Both consume the same commands as other callers.
For example, a first request can be:

```json
{"version":2,"id":1,"command":{"Workspace":{"Create":{"columns":80,"rows":24}}}}
```

Replies distinguish model application or admission from `Finished`. Input is
finished only when its complete encoded byte sequence is written to the PTY;
it does not claim application consumption. Explicit close removes model objects
first and finishes only after owned PTY/direct-child reclamation. A native
resize failure is a failed effect after model application and is reported
without claiming rollback. `inspect` delivers a complete screen and process
observation before finishing. Process facts include IO custody, retirement,
EOF, actual exit code/signal and current fault. Screens carry revisions,
styles/cursor/modes, and saturating byte, unsupported-sequence and bell counts.
`Screen.bytes` counts bytes accepted by the terminal engine, including controls
that make no visible cell change.

The runtime admits 32 connections and 32 panes, including retired panes until
reclamation. Session canvases are 2–256 columns and 1–64 rows, with at most
262,144 retained pane cells across the host. This cold-start host retains live
screens with zero scrollback; the terminal model's larger history profile is
separate. Model changes are previewed against runtime bounds before committing.
Primary/alternate screens, immutable snapshots and encoded messages have their
own bounded copies; this is not an exact process-RSS limit.

Each loop accepts at most four connections, handles one request and one 4096-byte
read per connection, writes at most 65,536 socket bytes per connection, and
per pane performs one 4096-byte application write, one query-reply write and one
4096-byte output feed. It serializes at most two presentations per turn, rotating
clients and panes and alternating catalog with terminal views. A 2 ms idle
pause is not a scheduling deadline; native spawn and kernel calls have no hard
wall-clock guarantee. The host never waits for a client to drain before driving
PTYs and control.

Requests are at most 128 KiB; presentation frames at most 8 MiB. Each connection
holds at most one active frame plus 64 queued reliable messages totaling 256 KiB.
Presentation is generated from current state only when that output slot is
available; intermediate views coalesce. After four reliable frames, a subscribed
connection yields one scheduling opportunity to presentation, so a continuous
request stream cannot indefinitely suppress its view. Partial frames are never
interleaved. There are at most 64 exact subscription
targets and 32 pending inspections per connection. Pane input holds at most
16 operations and 64 KiB of encoded bytes, retaining partial-write suffixes.
Terminal query replies have a separate 128 KiB queue; exhaustion backpressures
the PTY instead of discarding output. Admission refuses before queue mutation.

Malformed or saturated connections close independently. An unfinished connection
with no IO progress for ten seconds is closed. Reliable events are retained
within an admitted connection, not persisted across disconnect: loss of the
connection can leave a caller uncertain whether an effect completed. Inspect
current state before choosing further work; reconnect does not authorize replay.
Accepted input continues under host custody after disconnect. Graceful host
shutdown drains socket output for up to two seconds after native reclamation.

Source tests qualify the protocol codec, native socket/component ownership and
host composition. Formal independently distributed host/client application
scenes, pressure and installed behavior remain with the later Seatrial/release
leaves. No special Codex or tmux execution path exists.

## Interactive clients

The Linux x86_64 client requires a modern UTF-8 VT terminal with ANSI indexed/RGB
color, alternate screen, cursor positioning, line-wrap control, bracketed paste
and SGR mouse reports. This is an explicit terminal profile, not automatic
capability negotiation or qualification of every terminal emulator.
Wire v2 adds the host canvas bound to Hello and target-scoped pane subscription
revisions. Use a matching host/client build; wire v1 is not an accepted request
version in this source.

Run the explicit host described above. Open a new session with a complete
program specification, or join an existing session. `connect` observes by
default; `--take` explicitly acquires control. Optional `--fixed` with `--take`
preserves the existing host canvas while acquiring control, including from a
viewport too narrow for automatic acquisition.

```sh
bagua --socket /absolute/private/bagua/host.sock open \
  --executable /bin/sh --cwd / --arg=-i \
  --env PATH=/usr/bin:/bin --env TERM=xterm-256color
bagua --socket /absolute/private/bagua/host.sock connect 1
bagua --socket /absolute/private/bagua/host.sock connect 1 --take --fixed
```

`open` declares creation, joining, acquiring and launching as separate protocol
operations. A launch failure leaves its failed pane observable in the host.
Nothing inherits the host environment. Provide any additional application
variables explicitly. Supply the same optional executable/cwd/argv/environment
flags to `connect` when that client will create further panes. Split/window
creation from the interactive client refuses without an explicit program;
it does not infer a shell or reconstruct another process's launch data.

Press Ctrl-G followed by one command. This prefix is a Bagua choice, not a
tmux compatibility contract.

| Key | Action |
| --- | --- |
| `d` or `q` | Detach; hosted programs persist |
| `v` / `h` | Split into columns / rows and launch the configured program |
| `n` | Add a window and launch its first pane |
| Tab / `w` | Cycle panes / windows |
| `a` | Explicitly acquire released control |
| `t` | Transfer control to the first other joined client by ID |
| `f` / `u` | Fix the canvas to this viewport / resume automatic sizing |
| `x` / `!` | Explicitly close the focused pane / entire session |
| Arrows | Pan the local viewport without changing host geometry |
| `[` / `c` / `p` | Begin selection / copy / paste the local buffer |
| `?` | Show the command hints |

Ordinary keys go to the focused pane only for the controller. The profile
includes UTF-8 text, ASCII control keys, printable Alt characters, arrows,
navigation, F1–F12 and Backtab. Modified navigation/function keys and extended
keyboard protocols are refused. Escape uses a 50 ms disambiguation interval.
Bracketed paste accepts at most 4096 UTF-8 bytes; overflow is drained to its
terminator and refused, with bounded storage. Unsupported or oversized CSI
input is refused. The client uses a bounded decoder instead of an unbounded
event-reader paste accumulator.

Mouse presses focus panes for the controller. Mouse reports otherwise follow
the application's declared tracking mode and encoding. Shift-left-drag selects
locally in either role. Selection freezes one pane snapshot while the host and
live replica continue. In selection mode arrows extend the range, Escape cancels
and Ctrl-G c copies. Copy omits continuation cells and physical row padding,
joins soft wraps and preserves hard row breaks. The buffer is local to the
client, at most 4096 UTF-8 bytes, and is lost on exit. Ctrl-G p submits it through
the normal paste contract. No OS clipboard or OSC 52 effect is claimed.
Runtime scrollback remains zero; selection covers the current screen.

One terminal row is reserved for status. The local drawable viewport is bounded
to 512×255, with the host's narrower 256×64 canvas limits applied separately.
Observers crop and pan independently. A physical terminal smaller than two
columns/two rows suspends drawing and application input until resized; detach
remains available. Fixed canvas reporting never adopts observer geometry.
Cell contents are validated before installation; malformed control text and
foreign/retired panes cannot create a display authority. Wide cells crossing a
clip boundary render as blanks. Unsupported shaping widths also render as
blanks instead of shifting later cells. Styles are emitted from typed values;
arbitrary host escape bytes are never forwarded to the outer terminal.

The client reads at most 4096 TTY bytes and handles 16 decoded input events per
turn, and polls up to eight 65,536-byte transport reads. Keyboard staging is
8 KiB; paste storage is 4 KiB plus its terminator. A queue holds at most 64
unsent operations and 64 KiB of text/paste, batching adjacent unsent text for
one pane. At most 16 requests await completion; the channel independently bounds
64 frames/256 KiB. Budget refusal is explicit and does not claim host admission.
Detach drains queued operations and waits for results, with a ten-second bound;
an unfinished operation remains uncertain on timeout or connection loss.

Repaints are limited to one per 16 ms. Output holds one active frame and one
latest replacement, each at most 4 MiB; each turn writes at most 65,536 TTY bytes.
The client opens its own nonblocking `/dev/tty` descriptor, preserving inherited
file-description flags. Normal detach, startup errors and handled SIGINT/SIGTERM
restore raw mode and the selected display modes. Native tests compare the exact
before/after termios state. SIGKILL and a disappeared terminal are not normal
restoration paths. Client loss releases control while the host and its programs
continue. Source-native tests use real PTYs and the actual CLI against an
in-process Service; independent formal artifacts remain a later qualification.

## Terminal profile

The in-memory terminal uses the MIT-licensed
[vt100](https://github.com/doy/vt100-rust) engine. The initial dependency resolves
to 0.18.1, upstream source `fb0f54ec7222f4805f33cd3ca530229e0a0fe6ad`;
`Cargo.lock` owns the exact package resolution. No upstream source is copied.
Bagua disables its default features to keep VTE's OSC buffer at 1024 bytes.

The executable contract is `terminal::PROFILE`, `terminal::EXCLUDED` and the
terminal crate's integration tests. It selects cursor movement, erasing,
insert/delete, scrolling regions, primary/alternate screens, indexed/RGB
styles, bounded history, legacy keyboard input, bracketed paste and
legacy/SGR mouse reports. Status, cursor and character-size query replies are
returned as bytes; the model itself performs no IO. Observed unhandled
callbacks and bells are counted separately from API input refusals.

Screen dimensions are 2–512 columns and 1–256 rows, with at most 4096 history
rows. Columns multiplied by twice the screen height plus history must not
exceed 1,048,576 cells. Each feed or text/paste request accepts at most 4096
input bytes; callers retain and chunk larger streams. Query replies are
bounded by 16 times that feed budget. Rejected size/budget operations preserve
the model. Every accepted nonempty feed or changed size advances its revision;
an immutable snapshot binds content, modes and history to that revision.
History views do not mutate the live model. Clients own snapshot retention.

Streaming UTF-8, combining marks and CJK continuation cells are covered.
The engine stores at most 22 UTF-8 bytes per cell and discards excess combining
marks. Full grapheme shaping, emoji ZWJ and history reflow are excluded.
Wide characters that leave one vacant column cross a physical row boundary;
that boundary remains a newline in plain-text extraction. Normal autowrap
joins logical lines. DECAWM switching is unsupported; wrapping stays enabled.
Column changes truncate or extend physical rows. These choices require further
qualification with actual applications before the Linux product closes.

Input supports application-cursor arrows, navigation, F1–F12, explicit control
keys, UTF-8 text, bracketed paste and requested mouse tracking. Paste refuses
embedded controls except CR/LF/TAB, including any embedded ESC delimiter.
Legacy mouse coordinates stop at column/row 223; SGR uses the declared screen
bounds. UTF-8 mouse encoding is explicitly refused. Extended/modified keyboard
protocols are excluded; their negotiation is reported as unhandled. OSC
metadata, clipboard, images, hyperlinks,
window resize requests and other DCS extensions have no external effect.

The initial product targets Linux. macOS and Windows remain independently
unverified; a portable model does not establish native platform support.

See [AGENTS.md](AGENTS.md) for the product's working constraints and the
[PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
for contribution and acceptance policy.

Licensed under the [MIT License](LICENSE).
