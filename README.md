# Bagua

A persistent terminal workspace, built in the PerishLab domain.

Bagua is a wholehearted tribute to [tmux](https://github.com/tmux/tmux), with a few
engineering preferences of my own. Its mature design and practical lessons are
the foundation for this exploration.

Bagua explores explicit ownership of sessions, processes, terminal state and
client views. It follows executable contracts and bounded capability delivery.

The seven-crate workspace currently implements model contracts for control
ownership, geometry, process lifecycle and revision synchronization. It also
provides capability inspection and explicit refusal of unavailable operations.
A terminal interpreter, PTY host and interactive client are not implemented yet.

```sh
cargo run --locked -p bagua-cli -- capabilities --json
```

The binary uses the published PerishLab Plumb substrate. Building requires
access to the `perish` Cargo registry; registry declaration is included, while
credentials are supplied by the developer's environment. Unauthenticated
registry access has not been qualified. No private SDK source or credentials
are copied into this repository.

The initial product targets Linux. macOS and Windows remain independently
unverified; a portable model does not establish native platform support.

See [AGENTS.md](AGENTS.md) for the product's working constraints and the
[PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
for contribution and acceptance policy.

Licensed under the [MIT License](LICENSE).
