# Contributing

Each pull request should affect one package unless it intentionally coordinates
an API change across packages. Add or update the package README with its public
API, supported Kalcite version, and an executable usage example.

Before opening a pull request, run:

```sh
./scripts/check-packages.sh
```

Do not add generated `.kalcite/` caches, lockfiles from consuming projects, or
vendored dependencies. Keep package source portable unless its README clearly
documents a target-specific requirement.
