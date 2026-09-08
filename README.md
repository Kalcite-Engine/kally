# Kally

Kally is the Git-first package manager for Kalcite. It keeps requested
dependencies in `kally.toml`, resolves Git branches or tags once, and records
the immutable commit and a deterministic source checksum in `kally.lock`.

The reusable libraries themselves live in the separate
[kalcite-pkgs](https://github.com/Kalcite-Engine/kalcite-pkgs) monorepo, not
in this repository.

For a profile-based installation of Kalcite and Kally, use
[Kallyup](https://github.com/Kalcite-Engine/kallyup).

## Install

```sh
cargo install --git https://github.com/Kalcite-Engine/kally.git
```

## Use

```sh
kally init
kally add hash \
  git:https://github.com/Kalcite-Engine/kalcite-pkgs.git#packages/hash \
  main
kally sync
```

`kally init [DIR]` creates an empty `kally.toml` and refuses to overwrite one
that already exists. It can create the requested project directory, but does
not create a lockfile or contact Git until the first dependency is added.

`kally update tween` is the only command that advances a locked Git package.
`kally sync` materializes the exact locked source under `.kally/packages/`.
For CI and reproducible builds, use `kally sync --locked`: it rejects any
manifest/lock divergence or missing checksum and never resolves a Git reference
or rewrites `kally.lock`.
When an exact package cache is already available, `kally sync --locked --offline`
performs the same manifest and checksum validation without creating files,
contacting Git, or resolving a reference.
`kally status` is read-only: it audits the manifest, lockfile, and local cache,
returning a non-zero status when `sync` or an explicit `update` is required.
Use `kally status --json` for a stable report consumable by CI, the Kalcite
editor, or other tooling:

```json
{"healthy":true,"packages":[{"name":"hash","status":"ready"}]}
```

`kally clean --dry-run` lists stale named entries in `.kally/packages/` without
changing anything. `kally clean` removes only those entries that are absent
from `kally.lock`; it never removes a locked package.

## Design

Kally performs filesystem access, TLS and Git subprocess execution in Rust.
Lockfile grammar, source policy, resolution decisions and checksum transitions
are compiled from `src/kally_core.klc` and executed by Kally itself. The KLC
core uses the allocation-free `Text.equals` intrinsic to compare locked Git
sources and references, including bounded strings with different capacities and
fixed text literals such as the local-source reference.
Source classification likewise uses `Text.starts_with` for the `git:` and
`path:` prefixes while preserving Kally's non-empty payload requirement.
The lockfile key classifier uses the same KLC text operations for its exact
`version=1` header and `source=`, `reference=`, `revision=`, and `checksum=`
properties; Rust receives only the resulting validated line kind.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).
