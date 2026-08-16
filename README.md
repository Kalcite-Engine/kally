# Kally

Kally is the Git-first package manager for Kalcite. It keeps requested
dependencies in `kally.toml`, resolves Git branches or tags once, and records
the immutable commit and a deterministic source checksum in `kally.lock`.

The reusable libraries themselves live in
[kalcite-packages](https://github.com/Kalcite-Engine/kalcite-packages), not in
this repository.

## Install

```sh
cargo install --git https://github.com/Kalcite-Engine/kally.git --branch manager-core
```

## Use

```sh
kally add tween \
  git:https://github.com/Kalcite-Engine/kalcite-packages.git#packages/tween \
  main
kally sync
```

`kally update tween` is the only command that advances a locked Git package.
`kally sync` materializes the exact locked source under `.kally/packages/`.

## Design

Kally performs filesystem access, TLS and Git subprocess execution in Rust.
Lockfile grammar, source policy, resolution decisions and checksum transitions
are compiled from `src/kally_core.klc` and executed by Kally itself.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).
