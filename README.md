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

## Daily-use entry

Use the checked-in `bagua.toml` as an explicit profile in a modern terminal
connected to Linux. Review it and validate the selected file before entering:

```sh
bagua --config /absolute/path/bagua.toml config
bagua --config /absolute/path/bagua.toml enter workspace
```

Normal entry creates the exact named workspace if absent and requests control.
After detaching, the same entry reconnects to its existing host and programs.
`enter workspace --observe` requires an existing workspace and never acquires
control. A default endpoint is discovered only for normal entry/serve/config;
machine commands require an explicit socket or a profile containing `host.socket`.
Never remove an unknown endpoint to make a connection succeed.

The example selects this profile; compiled defaults still use Ctrl-G:

| Example binding | Action |
| --- | --- |
| Ctrl-Q twice | Send literal Ctrl-Q to the application |
| Alt-Q, or Ctrl-Q then d | Detach and restore the terminal |
| Ctrl-Q then `\` / `-` | Split columns / rows using foreground cwd |
| Ctrl-Q then `=` | New window using foreground cwd |
| Alt-1 through Alt-9 | Select the current 1-based window index |
| Ctrl-Q then Tab / w | Next pane / next window |
| Ctrl-Q then PageUp / End | Browse history / return live |
| Ctrl-Q then r | Reload all validated client preferences atomically |
| Ctrl-Q then x | Close the focused pane and reclaim its owned process |

Mouse drag on a divider changes the real application geometry. Wheel reads
history when application tracking is off; Shift-wheel requests local history
when tracking is active. Drag-release copies the current frozen page under the
configured policy. The example explicitly opts into OSC52; emitted encoding
is not proof that ET or the OS clipboard accepted it. Enter and ShiftEnter are
distinct when the hosted application negotiates supported keyboard flag 1,
including compound requests 5/7; unsupported flags are not advertised.

Use an independent outer terminal for initial human verification: another
multiplexer using Ctrl-Q may intercept the example prefix. Detach preserves
applications; closing panes or explicitly shutting down a known owned host is
a different operation. This profile does not migrate or stop tmux sessions.
Earlier v0.1.0 and v0.2.0 cannot connect to the current wire8
host. Rollback uses its own matching executable, documented old CLI and separate
endpoint; it never replaces an unknown live host or promises history recovery
after host restart.

## Distribution and runtime

The first distribution target is Linux x86_64 GNU, delivered as a `tar.gz`
containing one executable named `bagua` at its root. No resource directory,
runtime Cargo invocation, development checkout or internal Rust SDK is needed.
macOS, Windows, musl and package-manager media are not declared distribution
targets. Automatic host startup belongs to the daily-use interface below. The earlier
v0.1.0 requires explicit startup and cannot consume this configuration or wire
version. Use a matching release artifact for each host and its clients. MIT licensing covers Bagua's source; dependencies
retain their own licenses.

The release authority is `https://releases.bagua.perish.uk`. Plumb defines the
immutable marker, and Wharf builds, binds and publishes the executable. A
declaration or tag alone is not an available release. For an exact non-stable
marker, require the authority's
`v1/releases/<channel>/<marker>/distribution.json` to report `state: complete`
for that repository, commit, tree and publication attempt. Its matching
`seal.json` binds the archive's URL, size and SHA-256. Verify these before
extracting into a private directory; its `bagua` is the executable entry point.
`bagua -V` reports the bound marker, while an ordinary development build reports
its source build identity. Use the same artifact for the host and its clients.

Installing an upgrade does not migrate a live host or its applications. A host
keeps its original executable identity and wire contract. Preserve that matching
binary/profile to return to an existing workspace, and use a fresh private
endpoint for the new build while it remains active. Takeover transfers control
within one compatible host; it does not convert old sessions or restart their
programs. Keep the previous version and profile available for rollback.

The selected ABI requires the Linux ELF loader, glibc and the executable's
declared native shared libraries. Wharf builds the GNU target on its declared
Linux runner; this is not a static or arbitrary-old-glibc portability promise.
For the exact artifact, retain its ELF interpreter, `DT_NEEDED` entries and
required symbol versions as runtime dependency evidence. PTYs require a usable
`/dev/ptmx` and `devpts`; interactive clients also require their own controlling
`/dev/tty` and the terminal profile below. Hosted executables, their dependencies,
working directories and environment are explicit consumer inputs. Bagua does
not bundle a shell, terminal emulator, terminfo database or application.

Archive extraction does not start a host or alter PATH. The marker's seal also
references Wharf's generated manager for versioned installation. Non-stable
installation requires an exact marker plus explicit, isolated `--install-root`
and `--bin-dir` paths outside the stable defaults (`~/.local/share/bagua` and
`~/.local/bin`). Keep the executable installation apart
from the private runtime directory. The foreground `serve` command receives an
absolute socket path under an existing, canonical, user-owned mode-0700
directory. Ordinary named entry can select and create its private runtime
directory as described below; the socket is its only runtime file. No session
persistence or runtime log is created there. Use a fresh directory for each isolated scene, and explicit
homes/data for its hosted programs. No production session is a test fixture.

## Published-artifact qualification

`plumb.toml` requires all ten Linux cases below. The official Seatrial runner
and complete Depot Scene generation follow stable; the Images runtime follows
stable. Exact source, marker, publication attempt, archive, effective inputs,
runner, Scene generation and image identities belong to each qualification.
A declaration, source check or complete distribution does not imply a pass.
The v0.2.0 stable delivered wire7 with its historical nine-case qualification.
Current source expresses wire8 and the additional interaction contract; it needs
its own complete ten-case non-stable pass and content association before stable
promotion. Historical v0.2.0 evidence does not qualify this changed boundary.

| Required case | Public behavior to qualify |
| --- | --- |
| `bagua-workspace` | Sessions, explicit launch, layout/focus, control transfer, observer refusal, sizing, real PTY input/copy, detach and terminal restoration |
| `bagua-lifecycle` | Client loss preserves applications; exit/EOF/closure differ; launch failure and owned native reclamation remain observable |
| `bagua-flow` | Exact flood bytes/bells, fragmented UTF-8, slow/lost readers, full resynchronization, admitted input custody, tiny viewports and capacity recovery |
| `bagua-config` | Strict discovery/validation, redacted diagnostics, failed whole reload and successful client preference reload |
| `bagua-entry` | Named/default entry and host races, identity/unknown-endpoint refusal, foreground cwd inheritance, immediate input routing and deleted-cwd refusal |
| `bagua-history` | Full 50000-row reads at 80/160 columns, retention/byte budgets/fair share, scope/epoch/eviction refusal, frozen pages, keyboard/wheel navigation and reconnect |
| `bagua-input` | Actual application bytes for keyboard flags 1/5/7, stack/reset boundaries, prefix/release/paste, ordered layout input, 32-window/index coverage and known refusal without Escape or replay |
| `bagua-mouse` | Actual divider widths/focus, observer refusal, local/manual/release copy, Unicode/base64/ST, limit refusal and blocked application OSC |
| `bagua-display` | Authoritative header/content height, observed cwd, decor reload, small-view feedback, complete paint output and bounded refresh traffic |
| `bagua-interaction` | Invalid-index/no-op continuity, layout/native failure isolation, cross-fragment paste, Copy/Live/Esc, explicit takeover and owner races, unknown-outcome synchronization/exit without replay, TTY and application survival |

Seatrial owns the independently published black-box programs. They drive the
public CLI and declared wire version through case-owned PTYs, programs, sockets
and files. There is no product source checkout, private crate link, runtime
Cargo build or product testing switch. Required cases cannot be dropped to
manufacture qualification. Preparation dependencies are not automatically
verified subjects. Seatrial #83/#88 record the daily-use material lineage;
Bagua #31 and Seatrial #84 retain v0.2.0 candidate publication and execution.
Seatrial #101 owns the new interaction material; Bagua #53 owns its containing
candidate/stable qualification, complete distribution and installed consumption.

Inspect the immutable attempt under
`v1/releases/<channel>/<marker>/windtun/` and use `plumb windtun check` to verify
current public qualification. All declared cases and successful lane cleanup
are required. Missing, pending, failed, changed or unsupported evidence remains
nonpositive. Qualification, content association, stable promotion and installed
consumption are separate facts. This declaration alone establishes none of them.
Linux automated fixtures do not establish macOS/Windows support, ET behavior,
the actual OS clipboard or subjective daily comfort; independent Bagua #33
retains those human checks.

## User configuration

The daily-use interface includes `bagua config`, `--config PATH` and
`--no-config`. The earlier v0.1.0 stable does not contain these commands;
use a containing, qualified artifact for installed consumption. The checked-in
[bagua.toml](bagua.toml) is a runnable example, not an automatically loaded
working-directory configuration.

An explicit `--config` selects exactly that file and refuses missing or invalid
input. Otherwise the CLI selects `$XDG_CONFIG_HOME/bagua/bagua.toml`, or
`$HOME/.config/bagua/bagua.toml` when XDG_CONFIG_HOME is unset or empty. The
directory must be absolute. A missing default file uses built-in defaults;
unreadable or malformed files refuse. `--no-config` bypasses file discovery.
No parent-directory search, shell expansion, script execution or BAGUA-prefixed
environment overrides occur. Relative paths supplied explicitly with --config
are resolved from that invocation, rather than silently searched.

```sh
bagua --config /absolute/path/bagua.toml config
bagua --config /absolute/path/bagua.toml --socket /absolute/private/host.sock serve
bagua --config /absolute/path/bagua.toml
```

Schema version 1 is the default when omitted. Supported sections are `client`, `host`, `history`, `workspace`, `bindings`,
`mouse`, `clipboard`, `header`, `status` and `program`. Unknown fields, duplicate keys and unsupported versions
refuse before interactive mode or native startup. `client.prefix` is a control
letter such as C-g or C-q; C-h/C-i/C-j/C-m are reserved because the decoder maps
them to navigation keys. `client.escape_ms` is 0–1000 milliseconds, default 50;
zero makes an isolated Escape immediate and may split fragmented Alt or CSI input.
The built-in prefix remains Ctrl-G. Pressing the chosen prefix twice sends its
literal control key to the application. Prefix-r reloads and reports its result.

Reload validates the whole file before atomically replacing client prefix,
Escape delay, bindings, mouse, clipboard, header and status preferences. Errors, missing reload input or changed host/program/history/workspace sections
preserve the old client configuration. Host socket, history budgets and program settings apply
only to a new invocation; they never restart a host or alter running programs.
Each client owns its preferences, including an observer. Preferences grant no
control; only a controller can adopt a header inset through a model operation. A disabled or undiscoverable source cannot reload.

`host.socket` supplies an absolute socket path; --socket overrides it.
`program` supplies the complete executable, argv, absolute cwd and explicit
environment for interactive open/connect and later splits/windows. A complete
CLI --executable/--cwd launch replaces that whole template, including argv and
environment. Nothing implicitly inherits the host environment. Values remain
literal data; add application HOME or other variables explicitly when needed.
Reloading a program template requires a new invocation even if that invocation
had overridden it with CLI launch data.

`bagua config` reports the selected source, whether it was loaded, the effective
socket, client settings and reload policy. It shows program paths and only
argument counts/environment key names. It neither starts programs nor proves
their executability, directory access or runtime permissions. Parse diagnostics
report field/byte information without echoing the original TOML source line.
Typed bindings, mouse, clipboard, header and status are described below.
Unknown sections/components refuse instead of executing shell formats.

## Ordinary named entry

Issue #26 adds this source workflow; #31/#32 own containing release and installed
qualification. A bare `bagua` requests control of the exact workspace named by
`workspace.name` (default `workspace`), creating the host and workspace if absent.
An existing controller opens an explicit Take / Observe / Cancel choice; it is never displaced implicitly.

```sh
bagua
bagua enter work
bagua enter work --existing
bagua enter work --observe
bagua enter work --existing --fixed
bagua --take
bagua enter work --take
bagua connect 1 --take
bagua --observe
```

`--existing` forbids creation. `--observe` requires an existing workspace and
neither starts a host nor acquires control. Explicit `--fixed` keeps the current
canvas while acquiring, allowing recovery from a small viewport. Ordinary named entry commits joining and acquisition atomically; refusal
preserves model identities, revision, membership and geometry. When occupied,
the chooser identifies the observed controller and viewport. Take is explicit,
validates the new client/viewport, and compares that expected owner before
transferring control atomically. A changed owner or invalid viewport refuses
without displacing the current controller. The old interactive client detaches
normally and restores its TTY; the host and application processes continue.
`--take` works consistently before/after `enter` and with `connect`; `--observe`
never requests control. Cancellation returns normally without joining.
An occupied-entry observer joins only after the explicit choice; a refused
explicit takeover may temporarily have joined as an observer, which its
connection cleanup removes. It grants no write authority before transfer. An empty workspace gets its first window;
an exited or failed pane remains inspectable and is never automatically restarted.

Names are exact, case-sensitive UTF-8, 1–64 bytes, without controls or surrounding
whitespace. Session names are unique in the host; window names in their session.
Machine `new --name NAME` and `rename session|window ID NAME` use these same rules;
rename requires control and keeps IDs unchanged. Automatic names choose a free
`session-N`/`window-N`. Window display indices start at 1 and close gaps after
closure; Alt-1…Alt-9 resolves the current display index to a stable pane ID.
There is no Alt-0 binding or nine-window model limit. Status prioritizes window
index, role and refusal feedback before session/window labels.

The default socket is `$XDG_RUNTIME_DIR/bagua/host.sock`, otherwise
`$XDG_STATE_HOME/bagua/host.sock`, otherwise
`$HOME/.local/state/bagua/host.sock`. Nonempty bases must be absolute. CLI
`--socket` overrides `host.socket`, which overrides this default. Only normal
entry and `serve` select it; machine operations do not silently discover a host.
Bagua creates its managed directory privately and requires the endpoint parent
to be canonical, owned by the current user and mode 0700. An explicit socket
uses an already prepared private directory. Unknown files, stale sockets,
symlinks and endpoints are never unlinked or replaced. Readonly entry does not
create a missing default directory.

When absent, normal entry makes one startup attempt by executing its own
kernel-bound `/proc/self/exe`, then waits up to five seconds for the endpoint.
A two-second Hello read checks wire version and the running executable's device
and inode before any workspace command. Byte-identical copies at another inode
also refuse; this is local file identity, not a content digest or release
signature. Use one installed executable path for host and clients. Competing
clients may both attempt startup; exclusive socket binding selects the host and
atomic workspace acquisition selects one controller. Timeout or lost wait
observation is an unknown outcome requiring endpoint inspection before retry.
Kernel calls themselves have no hard wall-clock guarantee.

Without a complete file/CLI program template, normal entry explicitly captures
the invocation's absolute `$SHELL` (or `/bin/sh`), argv `-i`, cwd and UTF-8
environment, setting `TERM=xterm-256color`. Invalid, non-UTF-8 or excessive launch
data refuses before host startup. The resulting complete program is sent as
data; the host never reconstructs it from another process's argv or environment.
The background host receives only HOME/PATH, XDG directory variables, locale and
TZ for its own startup; it does not inherit the invocation's other credentials.
Explicit `open`/`connect` keep their complete-template requirement. A `[program]`
section or complete CLI launch replaces the whole normal-entry default.

`workspace.cwd = "foreground"` is the default. Before creating a split or window,
the client queries the focused pane's native foreground process group. Linux
validates its owned PTY/session, same UID, live group leader and start identity,
then reads and checks its absolute UTF-8 cwd and search access. It rechecks group
identity before returning. Shell `cd` and foreground TUIs therefore select the
current directory. Exited, missing, unreadable, deleted or replaced directories
refuse before layout mutation. An empty workspace has no pane to query and
explicitly uses the launch cwd. `workspace.cwd = "launch"` always selects the
complete program template's cwd. This is a point-in-time observation; a later
filesystem race may still make native launch fail, leaving a visible failed pane.

Cwd requests hold their original queue position until resolved or refused;
later layout intents cannot pass them. Each queued intent captures its source
pane and complete launch data. Startup of a created pane precedes unsent input
to that pane. These source checks do not replace distributed-artifact races,
pressure or actual daily shell/Codex qualification in Seatrial #83/#84.

## Key bindings and modified input

Issue #28 adds source configuration and an end-to-end keyboard subset. These
changes still require the containing candidate and independent qualification.
Bindings are data, compiled by the client before host startup or atomic reload:

```toml
[client]
prefix = "C-q"
escape_ms = 50

[bindings.prefix]
'\' = "split-columns"
'-' = "split-rows"
'=' = "window"
r = "reload"

[bindings.direct]
Alt-q = "detach"
```

Prefix and direct tables overlay the documented defaults. An ordinary unbound
key goes to the application; an unknown key after the prefix refuses. `pass`
explicitly removes a command binding and submits that key as application input;
`ignore` consumes it locally. The prefix itself is reserved in both tables:
one press waits with feedback, two presses submit the literal control key using
the application's current encoding. Escape cancels the waiting prefix. A release
event does not complete it; a paste or decoder refusal cancels it. Paste text
never enters either key table, and embedded controls/escape delimiters still
refuse under the existing paste contract. Observers cannot gain application
write or workspace control through a binding.

Key names are one printable Unicode character, `Space`, `Enter`, `Tab`,
`Backspace`, `Escape`, arrows, `Home`, `End`, `PageUp`, `PageDown`, `Insert`,
`Delete` or `F1`–`F12`, optionally prefixed by `Ctrl-`, `Alt-`, `Shift-` once each.
`Shift-Tab` includes legacy Backtab. Ctrl-only ASCII letters normalize case.
Duplicate equivalent selectors, repeated modifiers, unknown keys/actions,
reserved prefix overrides and more than 128 total entries refuse. Actions are
`detach`, `split-columns`, `split-rows`, `window`, `window-1`–`window-32`, `pane`,
`next-window`, `acquire`, `transfer`, `fixed`, `automatic`, `close-pane`,
`close-session`, `select`, `copy`, `paste`, `help`, `reload`, `page-up`,
`page-down`, `live`, `left`, `right`, `up`, `down`, `pass` and `ignore`.
`bagua config` reports the overrides. Help identifies the active prefix and
labels its hints as defaults for keys without overrides. Complete validation
includes the changed prefix and key tables together; failed reload keeps both.
Bindings, prefix and Escape delay are client-local and never restart the host.

The outer decoder accepts bounded `CSI codepoint ; modifiers u` with optional
press/repeat/release suffix `:1`/`:2`/`:3`; press is the default and repeat acts
as a press. Releases are ignored rather than sent to applications. The modifier
number is 1 plus Shift=1, Alt=2 and Ctrl=4. It also accepts xterm modified
navigation/F1–F12. Unicode scalars, Enter/Tab/Backspace/Escape and the declared
navigation/function keys are supported; C1 controls, surrogate values, other
private function keys, lock/super/media/keypad keys, alternate-key fields and
associated-text fields refuse. Existing 64-byte sequence and 8 KiB staging
bounds apply. No input sequence is forwarded as unchecked raw bytes.

For the inner application, the terminal engine implements the flag-1
(disambiguation) subset of the [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/).
`CSI ? u` returns its current flags; `CSI > flags u`, `CSI < count u` and
`CSI = flags ; mode u` push, pop and set them. Push/set adopt only the supported
bit 1, including from combinations such as 5 or 7; query reports only actual
flags 0/1. A request containing unsupported bits increments unsupported once,
even when its supported bit is adopted. Set modes 1/2/3 replace/set/clear;
omitted/zero set mode uses replacement, and omitted/zero pop count means one.
Replacing or pushing with unsupported bits alone selects zero; set/clear apply
only bit 1. Pop uses a count, never a flag mask. Malformed forms and invalid
set modes increment unsupported without changing mode. Primary and alternate screens retain separate stacks,
each bounded to 16 entries including its current state; overflow evicts the
oldest entry. Exhausting a stack resets to zero; RIS clears both. Primary device
attributes (`CSI c`/`CSI 0 c`) reply `CSI ? 1 ; 2 c` to complete protocol detection.

Ordinary Enter remains CR. With flag 1, Shift+Enter becomes `CSI 13 ; 2 u`,
Ctrl/Alt/combined modified keys retain their distinctions, and Escape becomes
CSI-u. Modified navigation/F1–F12 retain unambiguous xterm encodings even in
legacy mode. Other modified keys that legacy encoding cannot distinguish
refuse; Shift+Enter is never silently collapsed into Enter. Text stays UTF-8
and paste keeps its own framing. The host encodes against the authoritative
current pane mode, so a delayed client view cannot force an obsolete encoding.
Full enhancement flags 2/4/8/16 and inner release reporting are outside this
contract; applications may query the supported result and fall back.

The interactive client pushes flag 1 in its own outer alternate screen, and
pops it before leaving that screen on normal/error cleanup. The inner application's
requests never pass through to the outer terminal. A legacy outer terminal can
continue sending its existing input, but cannot recover a distinction it never
encoded. Source PTY tests observe the application's exact query replies and
Enter/Shift+Enter bytes, withdrawal to legacy mode, configuration reload and
terminal restoration. Actual terminal emulator/Codex behavior and pressure
remain Seatrial qualification, rather than a full Kitty support claim.

## Input after layout and focus changes

Issue #40 fixes source input that could otherwise reach the old pane when text
immediately followed a split or focus shortcut. Keyboard and paste gestures
are kept without a pane target until preceding workspace mutations settle.
The client waits for their applied model revision in a validated catalog and
reliable completion; a new pane also waits for its native start result. Native
retirement is an accepted cleanup intent with an already applied model change,
and remains distinct from finished resource reclamation. No optimistic mutable
World is installed in the client. Ordinary viewport reports do not switch the
input target and need no focus barrier.

Deferred gestures are FIFO, at most 64 entries and 64 KiB, in addition to the
existing command and keyboard budgets. Adjacent unsent text may batch up to
4096 UTF-8 bytes. Once the gesture buffer lacks room for a complete decoded
event, input staging backpressures instead of consuming that event. Input that
has already acquired a target and reached command admission is never retargeted
or replayed. Multiple split/window/switch gestures therefore resolve against
the results of preceding gestures. Clipboard or history gestures also keep their
position; reload waits behind prior gestures, while help and detach remain
local controls. Detach drives previously captured gestures and admitted commands
to completion within its existing ten-second bound. A suspended tiny viewport
holds deferred gestures until it becomes drawable again.

Mouse handling first drains earlier keyboard intents. A click's focus command
precedes later keyboard target selection. A mouse gesture while a prior layout
is unresolved or earlier gestures still need draining refuses explicitly; its
coordinates are never silently interpreted against a different layout. Local
pointer policies are further specified by Issue #29.

A known layout/focus refusal or native start failure ends the affected intent
chain. Its dependent, unsent gestures are discarded instead of being sent to
the old pane. Fresh intentions continue without an Escape acknowledgement.
Intent sequence and dependency identities span TTY reads; a read or packet is
never a command group. An incomplete keyboard sequence or paste belonging to
the failed chain is consumed without delivering its remaining content.
Already admitted commands retain their exact targets and byte custody.

An invalid window index reports the available range and leaves focus unchanged.
Selecting the current window or cycling a single pane is a no-op. These local
navigation outcomes create no layout barrier and do not discard following text.
An unmodified Escape in Live goes to the application; while waiting for a
prefix it cancels that prefix.

An unconfirmed model outcome enters bounded recovery. New application input
is refused; Escape cannot confirm the outcome or authorize replay. After
outstanding reliable work settles, an explicit complete catalog read and its
completion establish the current model before fresh intentions resume. If
confirmation cannot finish within the recovery budget, only the client exits
and restores its terminal; the host and applications remain. Help, reload and
detach remain available. A lost connection never authorizes replay.

The own-CLI regression checks batch shortcuts and literal shell commands in one
PTY write, then independently inspect each pane. They cover split, new window,
Alt return, mouse focus, close/fallback and refusal without Escape;
pure protocol checks retain text until fresh model/start evidence and refuse
writes after transfer. These source checks do not replace Seatrial #83's
independent distributed-artifact workflow qualification.

## Bars and bounded templates

Issue #30 established the first header/status profile. Issue #75 separates bar
layout from pure custom templates; complete release and installed qualification
are tracked by #74. The product defaults and example select a light pane-border
title, a blank feedback row and a session/window navigation row. Legacy explicit
`components` selections keep their previous formatter.

A bar owns placement, reserved rows, alignment, clipping and refresh. Its templates
choose content and typed style roles from an installed state snapshot. Templates
perform no IO, environment lookup, shell execution or interpretation of dynamic
field text. This is Bagua syntax, without tmux format-language compatibility.

```toml
[header]
enabled = true
position = "top"
format = ' W{{window.index}}:{{window.title:24}} · P{{pane.index}} {{pane.command}} · {{pane.cwd_name}} '
border = true

[status]
position = "bottom"
timezone = "Asia/Shanghai"
refresh_seconds = 15

[[status.lines]]
fill = false

[[status.lines]]
left = '{{#if client.columns >= 64}} {{session.name:18}} {{/if}}{{#windows}}{{#if window.active}}{{style active}}{{else}}{{style inactive}}{{/if}} {{window.index}}:{{window.title:24}} {{style base}}{{/windows}}'
right = '{{#if client.columns >= 64}}{{clock.time}}  {{clock.date}} {{/if}}'
```

`header.position` and `status.position` accept `top` or `bottom`. A header reserves
one frame row, while each pane retains at least one content row. Status accepts
one to four lines, each with `left`, `center`, `right` and `fill` (default true).
An empty line with `fill=false` provides terminal-background spacing. Right text
has its width reserved first, center text stays between left and right, and all
text clips by Unicode display columns. Window lists select a range around the
active window and mark omitted neighbours. `separator=true` adds one line before
bottom status or after top status, preserving the old separator contract.

### Template grammar and context

- `{{field}}` inserts one named field. `{{field:18}}` clips it to 18 display
  columns (0..=512), without consuming the budget of subsequent text. `{{{{`
  inserts a literal `{{`.
- `{{#if field}}yes{{else}}no{{/if}}` selects by nonempty value; `else` is optional.
- Numeric fields also admit `{{#if client.columns >= 64}}...{{/if}}`: one
  lower-bound comparison to an unsigned literal. Other operators, expressions
  and comparisons against text fields refuse. The defaults limit session labels
  and hide them and the clock below 64 columns to retain window navigation.
- `{{#windows}}...{{/windows}}` repeats over a bounded window view; nested window
  lists refuse. Window and pane fields inside the list refer to that window and
  its focused pane. Lists retain the selected window when clipping is necessary.
- `{{style base}}`, `{{style active}}` and `{{style inactive}}` change the style
  of following text. Dynamic fields never become template instructions.

Fields are `session.name`; `window.index`, `window.name`, `window.title`,
`window.active`; `pane.index`, `pane.id`, `pane.command`, `pane.cwd`,
`pane.cwd_name`, `pane.active`, `pane.state`; `client.role`, `client.mode`, `client.columns`, `client.copy`, `client.recovering`, `client.observe`;
`prefix.key`, `prefix.active`; `notice.text`; and `clock.time`, `clock.date`.
Boolean fields are nonempty when true. Unknown fields refuse. Window/pane indices
are one-based presentation values; `pane.id` explicitly requests the stable ID.
Automatic window titles use available focused-program metadata and otherwise the
window name. An explicit window rename keeps its name regardless of that sample.

Each template is at most 4096 UTF-8 bytes and 128 syntax nodes, with nesting at
most eight. Literal controls/newlines, malformed blocks and unsupported styles
refuse. Use separate status lines for line breaks. Rendering sanitizes controls
from dynamic values, admits at most 32 windows and bounds output to 8192 bytes
and the available display columns. It neither executes nor reparses inserted
program names, directories, user labels or strings that resemble template tags.

Native command/cwd data is sampled only for visible running panes through owned
PTY context reads. At most two metadata jobs share the existing sixteen-job
client bound, after queued input/start work. Metadata expires after two refresh
intervals; missing or expired command/cwd fields become empty. Inactive windows
without a valid sample use their actual window name. Template evaluation never
triggers a read. This display cache never supplies the fresh cwd lookup for a
new split/window. Use a conditional for unavailable fields; the default header
does so. Legacy `cmd~`/`cwd~` retain their sampled meaning and `cmd?`/`cwd?` retain
their unavailable meaning.

### Configuration, feedback and geometry

Within each section choose explicit legacy `components` or new `format`/`lines`;
mixing them refuses. Legacy header components remain `pane`, `command`, `cwd`;
legacy status components remain `session`, `windows`, `prefix`, `notice`, `clock`.
Lists remain unique and contain at most eight values. Missing new format/lines
select the product default templates. `bagua config` reports the selected mode,
actual compiled templates, row count and desired theme, without claiming that a
host adopted the preferences.

Colors use indexed values 0..=255. `active`/`inactive` select background slots;
`highlight`/`foreground` select their foreground slots. Header `border=true`
places the colored label within a light line; false fills its whole frame row.
`color=false` uses terminal defaults and the existing active indication. Palette
appearance belongs to the outer terminal, without a pixel-identity claim.

The first status line is the system feedback region while needed: mode, observer
role, recovery, prefix waiting and notices take priority over its user template.
Empty templates cannot hide these facts. With multiple lines, navigation remains
on the other lines. Notices expire after two seconds without consuming input.

World owns header reservation and edge through atomic `Workspace.Decorate`.
Its placements expose frame and content rectangles used by native PTYs and grids.
Insufficient geometry refuses without changing actual layout. Observers follow
the actual inset and cannot apply their desired header edge. Status rows are
removed from the client viewport before layout; painting, cursor and mouse input
share the same top offset. Header clicks focus only for the controller; divider
geometry remains the outer frame. Cropping and panning retain their existing
bounds. Configuration reload compiles and validates the complete replacement
before changing keys, mouse, clipboard or decor; invalid input leaves the last
valid configuration. Native layout adoption is a separate fallible operation,
never permission to obscure application cells or claim an unconfirmed resize.

Clock fields use `UTC` or current `Asia/Shanghai` (UTC+08:00); this is not a
historical IANA/DST database. Refresh is 1..=3600 seconds, default 15. Date and
clock share that sample; scheduling/backpressure may delay presentation. No shell
timer runs. Header metadata uses the same refresh interval.

A paint contains complete validated rows and cursor state. The TTY writer
computes its difference against the last **fully written** paint, and commits
that baseline only when all frame bytes are written. Replaced latest paints
cannot become a false baseline. Unchanged pictures emit zero bytes, a clock-only
change rewrites one status row, and geometry changes request a full refresh.
Reliable OSC52 effects stay between frames and leave the display baseline intact.
The writer bounds the latest, active and committed paints to 4 MiB each, plus at
most 4 MiB for encoded active display bytes, alongside the independent reliable
copy budget. Per-turn writes and the 16 ms presentation interval remain bounded.

Source checks cover observation expiry, prefix/error coexistence, Unicode/long
fields, thirty-two windows, narrow/resize geometry, slow partial output with a
dropped intermediate paint and a reliable effect, and unchanged/single-row
traffic. Real own-CLI PTYs check application sizes, observed shell cwd changes,
atomic reload/adoption, tiny-terminal fallback and a sixty-update counter's
bounded output. These do not replace formal distributed-artifact/ET pressure
qualification or the user's final visual verification.

## History

History belongs to the daily-use interface. A containing distribution, current
independent qualification and actual installation remain distinct evidence;
v0.1.0 has no scrollback. Issue #27 records the source contract.
`[history]` configures `lines` (0–50000, default 50000), `bytes` (0–67108864,
default 67108864) and `total` (bytes–268435456, default 268435456). Only the
host's startup configuration chooses retention. A connecting client's file
cannot resize or reconfigure an existing host. Changes require a new host;
reload refuses them. `bagua config` reports the selected budgets.

Only primary screen rows that scroll off are retained. Alternate screen output
has no scrollback. History consists of physical rows with original styles,
column counts and soft-wrap flags, without reflow on resize. Growing the live
screen may introduce upstream blank rows; it does not restore history into the
application screen. Narrowing a live row keeps vt100's truncation behavior.
Already retained rows preserve all captured text; prefix Left/Right pans a
history page horizontally, up to the terminal's 512-column profile. Partial
wide cells at a page edge become blanks. CSI 3 J on the primary screen and RIS
clear stored history; old local selections remain independent frozen copies.

Prefix PageUp enters history; prefix PageUp/PageDown or ordinary PageUp/PageDown
while paused navigate by one pane height. Prefix End, ordinary End or Escape
returns live. Wheel moves three rows when application tracking is off; Shift-wheel
explicitly reads history even when tracking is active. An observer may read
history without acquiring control. Wheels target the focused pane. Ordinary
application input in Copy refuses until returning Live. Escape cancels both
selection and history browsing. Successful explicit copy and copy-on-release
return to Live immediately, so the next key reaches the application without an
extra Escape. A failed copy retains the selection and previous buffer. Ongoing
output neither changes the frozen page nor scrolls it. Focus/geometry changes
discard the frozen view and selection. Copy mode and its position remain visible
even after its transient notice expires.

Rows have monotonically increasing identities. Requests carry host context,
pane and geometry/reset epoch plus absolute row position. Foreign contexts,
old epochs and evicted ranges refuse; a new bootstrap read explicitly refreshes
the range. Reconnection discards the client's old view and starts fresh. Frozen
pages may remain visible after server eviction, but cannot silently acquire new
text under old row identities. Rows still on the live screen are mutable; only
rows below the returned `next` history boundary are stable retained identities.

History uses a shared segmented index and bounded row capture, independently
of vt100's zero-capacity scrollback. Model reads return at most 128 rows; wire
history reads return one pane-sized typed screen through the existing 8 MiB
presentation slot, followed by reliable completion. Full history is never part
of a live frame or connection bootstrap. History reads do not replay raw SGR,
OSC or PTY bytes to the outer terminal. Each client keeps one paused page and
one bounded selection copy, rather than accumulating downloaded history.

The host divides `total` equally among retained panes, including retiring panes,
and caps each share at `bytes`. Oldest rows are evicted first when either line
count or the effective share is exceeded. Defaults support 50000 ordinary
80/160-column text rows in a pane; richly styled/combining-heavy rows or many
full panes may hit the byte budget sooner. Returned pages expose actual retained
range, bytes and eviction count. Accounting includes string capacity and a
conservative row allocation allowance. Live grids, allocator overhead, shared
frozen snapshots and bounded transport buffers are additional memory: these
budgets are not an exact process-RSS limit. History eviction never discards
uninterpreted PTY output or reliable input/control events.

`cargo test --release -p bagua-terminal --test history -- --nocapture` writes
50000 full-width rows at both 80 and 160 columns, reads every retained row,
checks eviction and snapshot isolation, and reports capture/read and snapshot
cost. The client history test measures live/page serialization and 100 draws
at the same real capacity. Run each exact terminal test (`narrow` or `wide`)
as its own process when measuring RSS; compiler memory is separate. Local
timing varies with host contention and supplies no latency guarantee. These
source probes and the thin own-CLI PTY smoke do not replace formal distributed
artifact, shell/Codex, pressure or OS clipboard qualification.

## Workspace and control

The machine `Take` operation names a session, `expected` owner and optional
fixed geometry. `expected: null` means the session must currently be unowned,
never an unconditional override. Take removes the displaced client's membership;
ordinary owner-initiated Transfer retains its existing observer membership.
No age, idle-time or PID heuristic authorizes takeover. Admitted input keeps its
original targets; future writes from a displaced client refuse.

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
a pane collapses its split; an empty window is removed, and removing the last
window removes its session. Focus falls back to the lowest surviving ID.
New windows and splits select their newly created object. The host automatically
closes successfully started panes after process exit, PTY EOF, admitted input
completion and native reaping, for every exit code or signal. Startup failures
and native faults remain diagnosable. Attached clients leave a removed session
and restore their terminal; the host and other applications stay alive. A new
entry with the same name creates fresh identities and a new shell. This changes
the v0.3.0 behavior, which retained exited panes. Disconnect still releases
control without closing objects.

The model limits one world to 32 sessions and 4096 panes, one session to
32 windows and 64 attached clients, and one window to 64 panes and 16 tree
levels. Catalog snapshots own immutable bounded model state. Workspace geometry
uses nonzero 16-bit dimensions; the host must additionally enforce the terminal
engine's narrower profile before creating actual terminal resources.

CLI grammar, interactive request constructors and programmatic callers share
`protocol::Request`. Request numbers are nonzero; the default constructor uses
1 for a single request, while a persistent caller supplies its own numbers.
Connection identity is supplied by the host, never accepted from a command.
IDs and object kinds are exact. Session/window names are explicit metadata;
tmux target syntax is absent.

The pure `bagua::evaluate` composition applies model changes. Input and pane
retirement return accepted intents; they perform no native IO and do not
produce completion evidence. Native completion/failure, process outcomes and
presentation revisions have separate event types. Late outcomes for retired
IDs cannot recreate objects. Machine operations require an explicit or configured socket; normal `enter`
and `serve` may select the private default described below. Local capability inspection describes compiled endpoints, not whether
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
complete live snapshots and retains no incremental screen deltas. Scrollback is
read separately in bounded pages. Wire version 9 adds explicit header edges and automatic window-name state to expected-owner takeover to session header reservation, validated divider adjustments, named entry, native foreground-context reads, executable
file identity in Hello, keyboard mode flags and typed modified input. It uses a
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
{"version":3,"id":1,"command":{"Workspace":{"Create":{"columns":80,"rows":24}}}}
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
262,144 retained pane cells across the host. Source history defaults to 50000 primary physical rows per pane, bounded by
64 MiB per-pane and 256 MiB aggregate accounting; installed v0.1.0 has no history. Model changes are previewed against runtime bounds before committing.
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
targets, 32 pending inspections and one pending history read per connection. Pane input holds at most
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
Hello binds the host canvas and executable file identity; subscriptions bind
target-scoped revisions. Use a matching host/client build and wire version 9.
Installed v0.1.0 speaks wire 2 and cannot consume these source changes.

For explicit machine workflows, run the host described above. Open a new session with a complete
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

Press the configured prefix followed by one command (Ctrl-G by default).
This is a Bagua choice, not a tmux compatibility contract.

| Key | Action |
| --- | --- |
| `d` or `q` | Detach; hosted programs persist |
| `\` or `v` / `-` or `h` | Split into columns / rows and launch the configured program |
| `=` or `n` | Add a window and launch its first pane |
| Alt-1…Alt-9 (without prefix) | Select the corresponding current window display index |
| Tab / `w` | Cycle panes / windows |
| `a` | Explicitly acquire released control |
| `t` | Transfer control to the first other joined client by ID |
| `f` / `u` | Fix the canvas to this viewport / resume automatic sizing |
| `x` / `!` | Explicitly close the focused pane / entire session |
| Arrows | Pan the local viewport without changing host geometry |
| `[` / `c` / `p` | Begin selection / copy / paste the local buffer |
| `?` | Show the command hints |
| `r` | Validate and atomically reload client configuration |
| PageUp / PageDown / End | Page backward / forward through history / return live |

Ordinary keys go to the focused pane only for the controller. The profile
includes UTF-8 text, ASCII control keys, printable Alt characters, arrows,
navigation, F1–F12 and Backtab. Modified navigation/function keys use bounded typed input; the supported
keyboard enhancement subset and its refusals are described below. Escape uses a 50 ms disambiguation interval.
Bracketed paste accepts at most 4096 UTF-8 bytes; overflow is drained to its
terminator and refused, with bounded storage. Unsupported or oversized CSI
input is refused. The client uses a bounded decoder instead of an unbounded
event-reader paste accumulator.

Mouse presses focus panes for the controller. Without application tracking,
left-drag selects locally; Shift-left-drag selects locally even when the
application tracks mouse input. Observers can select without acquiring control.
Wheel events follow application tracking, or request host history when tracking
is off, Shift is held, or the focused pane is already browsing history. History
wheel routing requires the focused pane. Application presses retain their pane
rectangle until release; coordinates clamp to that rectangle. Local capture
cancels on changed geometry, retirement, out-of-viewport events, configuration
reload or refusal behind an earlier layout intention. Escape cancels a local
pointer gesture. A click without a drag leaves normal keyboard input available.

Dragging a divider shows its proposed ratio; release submits one `Adjust`
operation. World validates the exact previous divider (stable subtree anchors,
split and rectangle), current geometry and control authority before committing.
Invalid intermediate positions keep the last valid ratio. A stale or undersized
layout refuses atomically. The host then resizes its PTYs and terminal grids;
this never substitutes client-only geometry for application size. Observers
cannot resize. Shift bypasses divider capture.

Selection freezes one pane snapshot while the host and live replica continue.
Release copies and exits selection by default; `[mouse] copy_on_release=false`
retains manual selection. Arrows extend a manual selection, Escape cancels,
and the configured copy binding copies (prefix c by default). Copy includes a
whole wide cell when either column is selected, omits continuation cells and
physical row padding, joins soft wraps and preserves hard row breaks. It covers
the current live or frozen history page, without spanning unloaded pages.

```toml
[mouse]
enabled = true
resize = true
copy_on_release = true

[clipboard]
target = "local" # or "osc52", explicitly enabling outer-terminal writes
limit = 262144   # UTF-8 bytes, allowed range 1..=1048576
```

The local buffer is lost on exit. Oversized copies and a full pending-effect
queue refuse with a capacity error, preserve the previous buffer and never
truncate it. The configured paste binding (prefix p by default) still uses the
separate 4096-byte host paste contract; a larger copied buffer refuses paste.
Mouse and clipboard preferences reload atomically with keys and Escape delay.
`mouse.enabled=false` disables Bagua mouse handling, rather than forwarding
unchecked terminal reports. The example `bagua.toml` explicitly opts into OSC52.

`local` never emits OSC52. `osc52` also retains the local buffer, and enqueues
one standard-base64 `OSC 52 ; c ; data ST` effect only for a user copy. Application
OSC strings never pass through. The client retains at most four pending effects
and 2 MiB of encoded bytes; the terminal writer has an independent equal budget,
including its active effect. Full writer admission leaves the effect with the
client, without coalescing or duplication. Reload affects future copies;
already admitted effects finish in order. Detach drains pending client effects
under its existing ten-second bound. Terminal cleanup drains the active frame
and admitted effects within 250 ms before leaving display modes; failure reports
incomplete/unknown output and attempts parser cancellation and mode restoration.

One TTY writer completes a started frame before writing an effect. Latest-view
replacement cannot drop a copy or splice its bytes inside a VT sequence or UTF-8
cell. A queued or written OSC52 is not evidence that the OS clipboard changed.
An outer terminal, ET hop or nested multiplexer may refuse or filter it. Bagua
does not synthesize tmux passthrough wrappers or read the OS clipboard back.
Use an independent terminal for the trial, explicitly enable the appropriate
outer-terminal policy, and verify actual copy/paste manually in #33. Formal
candidate/ET qualification remains separate from the source-native PTY checks.

One to four template rows are reserved for status (one with legacy components), plus the optional separator row.
The local drawable viewport is bounded to 512 columns and 256 minus those reserved rows, with the host's narrower 256×64 canvas limits applied separately.
Observers crop and pan independently. A physical terminal smaller than two
columns or without room for the configured status and content rows suspends drawing and application input until resized; detach
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

Repaints are limited to one per 16 ms. Output holds active/latest/committed paints and the encoded active frame
under the independent byte bounds described above, plus the reliable-copy budget; each turn writes at most 65,536 TTY bytes.
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

Screen dimensions are 2–512 columns and 1–256 rows. Twice the live screen area
must not exceed 1,048,576 cells. History independently admits at most 50000 rows
and 64 MiB of accounted storage. Each feed or text/paste request accepts at most 4096
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
Column changes truncate or extend live physical rows; already retained history
keeps its original text and column count. These choices require further
qualification with actual applications before the Linux product closes.

Input supports application-cursor arrows, navigation, F1–F12, explicit control
keys, the keyboard enhancement subset below, UTF-8 text, bracketed paste and
requested mouse tracking. Paste refuses
embedded controls except CR/LF/TAB, including any embedded ESC delimiter.
Legacy mouse coordinates stop at column/row 223; SGR uses the declared screen
bounds. UTF-8 mouse encoding is explicitly refused. Keyboard enhancement requests adopt only the declared supported mask;
unimplemented bits remain counted as unsupported and are never reported as enabled. OSC
metadata, clipboard, images, hyperlinks,
window resize requests and other DCS extensions have no external effect.

The initial product targets Linux. macOS and Windows remain independently
unverified; a portable model does not establish native platform support.

See [AGENTS.md](AGENTS.md) for the product's working constraints and the
[PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
for contribution and acceptance policy.

Licensed under the [MIT License](LICENSE).

### Optional Locus observation

Observation is off by default. To enable it for a new invocation, add this data
section to the selected `bagua.toml`:

```toml
[locus]
enabled = true
```

The endpoint uses `locus.endpoint`, then `LOCUS_API`, then the shared local default
`http://127.0.0.1:43308`. `--no-config` disables observation. Changing `[locus]`
requires a new invocation; reloading does not reconfigure a running host. Existing
hosts retain their original settings and sessions.

The shared Locus reporter keeps its bounded spool under
`$XDG_STATE_HOME/bagua/locus`, or `$HOME/.local/state/bagua/locus`. The final directory
must be canonical, owned by the current user and private (0700). A pre-existing
unsafe directory is refused for observation without changing its permissions.
This persistent state is independent of the host socket and session cleanup.
No Bagua-specific file logger, rotation or retention settings are added.

Events cover connection-scoped structural request results, actual host process
exit/reaping, EOF, native resource release, pane retirement, client pending work
at return and confirmed terminal restoration. They contain stable codes and numeric
facts. Input, PTY output, clipboard data, launch arguments/environment/cwd, names
and raw error messages are excluded. Verified bound release identity accompanies
the invocation context; source builds keep unknown release identity.

An observation failure preserves business stdout and exit status. Diagnostics use
bounded stable codes on stderr, as JSON lines with `--json`, after confirmed TTY
restoration. Unknown restoration suppresses these diagnostics. A background host
with discarded stderr has no persistent engine-diagnostic channel. Missing events
prove neither successful cleanup nor the cause of an error; an unavailable service,
queued spool, process death and service retention can leave different gaps.

These are source behavior boundaries. Formal Bagua/Locus combined qualification
and the containing stable/install evidence are tracked in #70 and Seatrial #111;
this description does not establish their completion or replace human validation.
