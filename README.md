# Kalcite Packages

Official Git package monorepo for Kalcite projects. Each directory under
[`packages/`](packages) is independently consumable through Kally; the package
name is its directory name.

```sh
kally add tween \
  git:https://github.com/Kalcite-Engine/kalcite-packages.git#packages/tween \
  main
```

Kally locks the selected Git reference to an immutable commit in the consuming
project's `kalcite.lock`. Consumers therefore build from the locked commit;
only `kally update` moves a dependency forward.

## Repository layout

```text
packages/
  <package>/
    README.md       Package purpose, API and compatibility notes
    scripts/        Kalcite `.klc` source files
```

Packages must remain independent: no package may depend on unpublished local
state or on another package's source by relative path. Use one Git commit and
tag the repository for coordinated compatible releases.

## Status

`tween` is the first optional KLC package. Core language/runtime facilities
remain in the main Kalcite repository; this repository is for reusable,
non-primary libraries that can evolve independently.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).
