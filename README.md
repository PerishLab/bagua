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
operations. A PTY host and interactive client are not implemented yet.

```sh
cargo run --locked -p bagua-cli -- capabilities --json
```

The binary uses the published PerishLab Plumb substrate. Building requires
access to the `perish` Cargo registry; registry declaration is included, while
credentials are supplied by the developer's environment. Unauthenticated
registry access has not been qualified. No private SDK source or credentials
are copied into this repository.

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
