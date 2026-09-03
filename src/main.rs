use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn usage() {
    eprintln!(
        "usage:\n  kally add NAME git:URL[#SUBDIR] [BRANCH_OR_TAG] [DIR]\n  kally update [NAME] [DIR]\n  kally sync [DIR]\n  kally status [DIR]\n  kally remove NAME [DIR]\n  kally lock [DIR]\n\nKally manages Git packages for Kalcite projects. add resolves a branch or tag\nto an immutable commit in kally.lock; update is the only command that\nadvances a locked Git dependency. status is read-only and audits the local cache."
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let Some(command) = args.get(1).map(String::as_str) else {
        usage();
        return ExitCode::FAILURE;
    };
    match command {
        "add" => kally_add_command(&args[2..]),
        "update" => kally_update_command(&args[2..]),
        "sync" => kally_sync_command(&args[2..]),
        "status" => kally_status_command(&args[2..]),
        "remove" => kally_remove_command(&args[2..]),
        "lock" => kally_lock_command(&args[2..]),
        "help" | "--help" | "-h" => {
            usage();
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("unknown Kally command {command}");
            usage();
            ExitCode::FAILURE
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageStatus {
    Ready,
    Missing,
    ChecksumMismatch,
    Unresolved,
    Diverged,
    Stale,
}

impl PackageStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Missing => "missing",
            Self::ChecksumMismatch => "checksum-mismatch",
            Self::Unresolved => "unresolved",
            Self::Diverged => "diverged",
            Self::Stale => "stale",
        }
    }

    fn healthy(self) -> bool {
        self == Self::Ready
    }
}

fn kally_status_command(args: &[String]) -> ExitCode {
    let root = match args {
        [] => PathBuf::from("."),
        [root] => PathBuf::from(root),
        _ => {
            eprintln!("usage: kally status [DIR]");
            return ExitCode::FAILURE;
        }
    };
    let manifest = match kally::load_manifest(&root.join("kally.toml")) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("{}: {error}", root.join("kally.toml").display());
            return ExitCode::FAILURE;
        }
    };
    let lock = match kally::load(&root.join("kally.lock")) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("{}: {error}", root.join("kally.lock").display());
            return ExitCode::FAILURE;
        }
    };
    let mut names = std::collections::BTreeSet::new();
    names.extend(manifest.packages.keys().cloned());
    names.extend(lock.packages.keys().cloned());
    if names.is_empty() {
        println!("no Kally packages declared");
        return ExitCode::SUCCESS;
    }
    let mut healthy = true;
    for name in names {
        let status = package_status(
            &root,
            &name,
            manifest.packages.get(&name),
            lock.packages.get(&name),
        );
        healthy &= status.healthy();
        println!("{name}\t{}", status.label());
    }
    if healthy {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "run `kally sync` to materialize missing packages, or `kally update NAME` for declared changes"
        );
        ExitCode::FAILURE
    }
}

fn package_status(
    root: &Path,
    name: &str,
    requested: Option<&kally::ManifestPackage>,
    locked: Option<&kally::Package>,
) -> PackageStatus {
    let (Some(requested), Some(locked)) = (requested, locked) else {
        return if requested.is_some() {
            PackageStatus::Unresolved
        } else {
            PackageStatus::Stale
        };
    };
    match kally::resolution_action(requested, Some(locked)) {
        Ok(kally::ResolutionAction::Diverged) | Err(_) => return PackageStatus::Diverged,
        Ok(kally::ResolutionAction::Resolve) => return PackageStatus::Unresolved,
        Ok(kally::ResolutionAction::Keep) => {}
    }
    let cached = root.join(".kally/packages").join(name);
    if !cached.is_dir() {
        return PackageStatus::Missing;
    }
    if locked.checksum.is_empty() {
        return PackageStatus::ChecksumMismatch;
    }
    match kally::checksum_path(&cached) {
        Ok(checksum) if checksum == locked.checksum => PackageStatus::Ready,
        Ok(_) | Err(_) => PackageStatus::ChecksumMismatch,
    }
}

fn kally_lock_command(args: &[String]) -> ExitCode {
    let root = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let path = root.join("kally.lock");
    let lock = match kally::load(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = kally::save(&path, &lock) {
        eprintln!("{}: {e}", path.display());
        return ExitCode::FAILURE;
    }
    println!(
        "locked {} packages in {}",
        lock.packages.len(),
        path.display()
    );
    ExitCode::SUCCESS
}

fn kally_add_command(args: &[String]) -> ExitCode {
    if args.len() < 2 {
        eprintln!("usage: kally add NAME git:URL[#SUBDIR] [BRANCH_OR_TAG] [DIR]");
        return ExitCode::FAILURE;
    }
    let name = &args[0];
    if !kally::valid_name(name) {
        eprintln!("invalid package name `{name}`; use ASCII letters, digits, '-' or '_'");
        return ExitCode::FAILURE;
    }
    let source = &args[1];
    let reference = args
        .get(2)
        .filter(|x| !x.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "main".into());
    let root = args
        .get(3)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let path = root.join("kally.lock");
    let mut lock = match kally::load(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let source_kind = match kally::source_kind(source) {
        Ok(kind) => kind,
        Err(error) => {
            eprintln!("package `{name}`: {error}");
            return ExitCode::FAILURE;
        }
    };
    let (revision, checksum) = match source_kind {
        kally::SourceKind::Path => match kally::source_payload(source)
            .and_then(|local| kally::checksum_path(&root.join(local)))
        {
            Ok(checksum) => ("local".into(), checksum),
            Err(error) => {
                eprintln!("package `{name}`: {error}");
                return ExitCode::FAILURE;
            }
        },
        kally::SourceKind::Git => match resolve_git_revision(&root, name, source, &reference) {
            Ok(revision) => (revision, String::new()),
            Err(error) => {
                eprintln!("package `{name}`: {error}");
                return ExitCode::FAILURE;
            }
        },
    };
    let locked_reference = match source_kind {
        kally::SourceKind::Path => "local".to_owned(),
        kally::SourceKind::Git => reference,
    };
    let manifest_path = root.join("kally.toml");
    let mut manifest = match kally::load_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("{}: {error}", manifest_path.display());
            return ExitCode::FAILURE;
        }
    };
    manifest.packages.insert(
        name.clone(),
        kally::ManifestPackage {
            source: source.clone(),
            reference: locked_reference.clone(),
        },
    );
    if let Err(error) = kally::save_manifest(&manifest_path, &manifest) {
        eprintln!("{}: {error}", manifest_path.display());
        return ExitCode::FAILURE;
    }
    lock.packages.insert(
        name.clone(),
        kally::Package {
            source: source.clone(),
            reference: locked_reference,
            revision,
            checksum,
        },
    );
    match kally::save(&path, &lock) {
        Ok(()) => {
            match sync_kally_packages(&root).and_then(|_| lock_kally_checksums(&root, Some(name))) {
                Ok(()) => {
                    println!("added and locked package `{name}` at its resolved revision");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("package `{name}` was added but could not be synced: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Update Git packages by resolving their declared branch or tag again. The
/// lockfile records the resulting commit, so normal builds remain reproducible.
fn kally_update_command(args: &[String]) -> ExitCode {
    let (wanted, root) = match args {
        [] => (None, PathBuf::from(".")),
        [one] if one.starts_with('.') || Path::new(one).is_dir() => (None, PathBuf::from(one)),
        [one] => (Some(one.as_str()), PathBuf::from(".")),
        [name, root] => (Some(name.as_str()), PathBuf::from(root)),
        _ => {
            eprintln!("usage: kally update [NAME] [DIR]");
            return ExitCode::FAILURE;
        }
    };
    let path = root.join("kally.lock");
    let mut lock = match kally::load(&path) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("{}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let manifest_path = root.join("kally.toml");
    let manifest = match kally::load_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("{}: {error}", manifest_path.display());
            return ExitCode::FAILURE;
        }
    };
    let mut updated = 0;
    for (name, package) in &mut lock.packages {
        if wanted.is_some_and(|wanted| wanted != name) {
            continue;
        }
        let requested = manifest.packages.get(name);
        let source = requested
            .map(|package| package.source.as_str())
            .unwrap_or(&package.source);
        if kally::source_kind(source) != Ok(kally::SourceKind::Git) {
            continue;
        }
        let reference = requested
            .map(|package| package.reference.clone())
            .unwrap_or_else(|| {
                if package.reference.is_empty() {
                    package.revision.clone()
                } else {
                    package.reference.clone()
                }
            });
        if let Some(requested) = requested {
            package.source = requested.source.clone();
            package.reference = requested.reference.clone();
        };
        match resolve_git_revision(&root, name, &package.source, &reference) {
            Ok(revision) => {
                package.revision = revision;
                package.checksum.clear();
                updated += 1;
            }
            Err(error) => {
                eprintln!("package `{name}`: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    if wanted.is_some() && updated == 0 {
        eprintln!("no Git package named `{}`", wanted.unwrap());
        return ExitCode::FAILURE;
    }
    if let Err(error) = kally::save(&path, &lock) {
        eprintln!("{}: {error}", path.display());
        return ExitCode::FAILURE;
    }
    match sync_kally_packages(&root).and_then(|_| lock_kally_checksums(&root, wanted)) {
        Ok(()) => {
            println!("updated {updated} Git package(s)");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
fn kally_remove_command(args: &[String]) -> ExitCode {
    let Some(name) = args.first() else {
        eprintln!("usage: kally remove NAME [DIR]");
        return ExitCode::FAILURE;
    };
    let root = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let path = root.join("kally.lock");
    let mut lock = match kally::load(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if lock.packages.remove(name).is_none() {
        eprintln!("package `{name}` is not locked");
        return ExitCode::FAILURE;
    }
    let manifest_path = root.join("kally.toml");
    let mut manifest = match kally::load_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("{}: {error}", manifest_path.display());
            return ExitCode::FAILURE;
        }
    };
    manifest.packages.remove(name);
    if let Err(error) = kally::save_manifest(&manifest_path, &manifest) {
        eprintln!("{}: {error}", manifest_path.display());
        return ExitCode::FAILURE;
    }
    match kally::save(&path, &lock) {
        Ok(()) => {
            println!("removed package `{name}`");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
fn kally_sync_command(args: &[String]) -> ExitCode {
    let root = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    match sync_kally_packages(&root) {
        Ok(count) => {
            println!("synced {count} packages");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn sync_kally_packages(root: &Path) -> Result<usize, String> {
    let lock = reconcile_kally_manifest(root)?;
    let cache = root.join(".kally/packages");
    fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    if let Err(error) = kally::verify(&lock, &cache) {
        // A missing cache is expected before sync; structural lock errors are not.
        if !error.contains("checksum mismatch") {
            return Err(format!("lockfile: {error}"));
        }
    }
    for (name, p) in &lock.packages {
        match kally::source_kind(&p.source).map_err(|error| format!("package `{name}`: {error}"))? {
            kally::SourceKind::Path => {
                let local = kally::source_payload(&p.source)?;
                let src = root.join(local);
                let dst = cache.join(name);
                kally::materialize(&src, &dst)
                    .map_err(|error| format!("package `{name}`: {error}"))?;
                if !p.checksum.is_empty() {
                    let got = kally::checksum_path(&dst)
                        .map_err(|error| format!("package `{name}`: {error}"))?;
                    if got != p.checksum {
                        return Err(format!("package `{name}`: checksum mismatch"));
                    }
                }
            }
            kally::SourceKind::Git => {
                let checksum = materialize_kally_git_package(root, name, p)
                    .map_err(|error| format!("package `{name}`: {error}"))?;
                if !p.checksum.is_empty() && checksum != p.checksum {
                    return Err(format!("package `{name}`: checksum mismatch"));
                }
            }
        }
    }
    // Materialization is the only point at which a source-tree checksum is
    // meaningful. Persist it before returning so a fresh-manifest sync creates
    // a fully reproducible lock in one command.
    lock_kally_checksums(root, None)?;
    Ok(lock.packages.len())
}

/// Reconcile declarative intent before materialization.  The KLC core decides
/// whether each entry is an exact lock reuse, an initial resolution, or a
/// forbidden divergence; this function only performs the selected host I/O.
fn reconcile_kally_manifest(root: &Path) -> Result<kally::Lock, String> {
    let manifest_path = root.join("kally.toml");
    let lock_path = root.join("kally.lock");
    if !manifest_path.exists() {
        return kally::load(&lock_path);
    }
    let manifest = kally::load_manifest(&manifest_path)?;
    let mut lock = kally::load(&lock_path)?;
    let mut changed = false;
    for (name, requested) in &manifest.packages {
        match kally::resolution_action(requested, lock.packages.get(name))? {
            kally::ResolutionAction::Keep => {}
            kally::ResolutionAction::Diverged => {
                return Err(format!(
                    "package `{name}` differs from kally.toml; run `kally update {name}`"
                ));
            }
            kally::ResolutionAction::Resolve => {
                let (revision, checksum, reference) = match kally::source_kind(&requested.source)? {
                    kally::SourceKind::Path => {
                        let path = root.join(kally::source_payload(&requested.source)?);
                        (
                            "local".to_owned(),
                            kally::checksum_path(&path)?,
                            "local".to_owned(),
                        )
                    }
                    kally::SourceKind::Git => (
                        resolve_git_revision(root, name, &requested.source, &requested.reference)?,
                        String::new(),
                        requested.reference.clone(),
                    ),
                };
                lock.packages.insert(
                    name.clone(),
                    kally::Package {
                        source: requested.source.clone(),
                        reference,
                        revision,
                        checksum,
                    },
                );
                changed = true;
            }
        }
    }
    let before = lock.packages.len();
    lock.packages
        .retain(|name, _| manifest.packages.contains_key(name));
    changed |= lock.packages.len() != before;
    if changed {
        kally::save(&lock_path, &lock)?;
    }
    Ok(lock)
}

/// A checksum is calculated only after the exact locked Git commit has been
/// materialized. This keeps the lockfile a reproducible record of both the
/// selected commit and the package subtree copied into the project cache.
fn lock_kally_checksums(root: &Path, wanted: Option<&str>) -> Result<(), String> {
    let path = root.join("kally.lock");
    let mut lock = kally::load(&path)?;
    let mut changed = false;
    for (name, package) in &mut lock.packages {
        if wanted.is_some_and(|wanted| wanted != name) {
            continue;
        }
        let cached = root.join(".kally/packages").join(name);
        if cached.is_dir() {
            package.checksum = kally::checksum_path(&cached)
                .map_err(|error| format!("package `{name}`: {error}"))?;
            changed = true;
        }
    }
    if changed {
        kally::save(&path, &lock)?;
    }
    Ok(())
}

fn git_source(source: &str) -> Result<(&str, &str), String> {
    if !kally::git_source_valid(source) {
        return Err("Git source must be a safe `git:URL[#SUBDIR]` value".into());
    }
    let value = kally::source_payload(source)?;
    let (url, subdir) = value.split_once('#').unwrap_or((value, ""));
    Ok((url, subdir))
}

fn git_stage(root: &Path, name: &str, purpose: &str) -> PathBuf {
    root.join(".kally")
        .join(format!(".{name}-{purpose}-{}", std::process::id()))
}

fn run_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn fetch_git(root: &Path, stage: &Path, url: &str, reference: &str) -> Result<String, String> {
    if stage.exists() {
        fs::remove_dir_all(stage).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(stage.parent().ok_or("Git stage has no parent")?)
        .map_err(|error| error.to_string())?;
    let stage_text = stage.to_str().ok_or("Git stage path is not UTF-8")?;
    run_git(root, &["init", "--quiet", stage_text])?;
    let result = (|| {
        run_git(stage, &["remote", "add", "origin", url])?;
        run_git(stage, &["fetch", "--depth", "1", "origin", reference])?;
        run_git(stage, &["rev-parse", "FETCH_HEAD^{commit}"])
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(stage);
    }
    result
}

fn resolve_git_revision(
    root: &Path,
    name: &str,
    source: &str,
    reference: &str,
) -> Result<String, String> {
    let (url, _) = git_source(source)?;
    let stage = git_stage(root, name, "resolve");
    let result = fetch_git(root, &stage, url, reference);
    let _ = fs::remove_dir_all(stage);
    result
}

fn materialize_kally_git_package(
    root: &Path,
    name: &str,
    package: &kally::Package,
) -> Result<String, String> {
    let (url, subdir) = git_source(&package.source)?;
    let stage = git_stage(root, name, "fetch");
    let result = (|| {
        fetch_git(root, &stage, url, &package.revision)?;
        run_git(&stage, &["checkout", "--quiet", "--detach", "FETCH_HEAD"])?;
        let source = stage.join(subdir);
        if !source.is_dir() {
            return Err(format!(
                "package path `{subdir}` is not a directory in the selected commit"
            ));
        }
        let destination = root.join(".kally/packages").join(name);
        kally::materialize(&source, &destination)?;
        kally::checksum_path(&destination)
    })();
    let _ = fs::remove_dir_all(stage);
    result
}
