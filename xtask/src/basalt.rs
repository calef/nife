//! **basalt's published packages, fetched by digest for the gates** (milestone 810 (`ripgrep` is
//! packaged in basalt and installed with `jig`), DECISIONS §297 (software reaches a nife system as a
//! package `jig` installs from basalt) item 3). Name: provisional,
//! `cargo xtask basalt-fetch`, minted 2026-10-13 (UTC) by lane `lane/ripgrep-from-basalt`.
//!
//! nife builds no `rg`. basalt does (its `recipes/ripgrep.toml`), and publishes one package per
//! architecture on a GitHub Release with an index at `basalt.nifeos.org`. calef ruled on 2026-10-13
//! (UTC) that nife's tests take that package rather than a build of their own: *"1 is preferable
//! because that tests jig and basalt's delivery mechanism as well."*
//!
//! So this reads [`PINNED_INDEX`], basalt's own index lines for the packages nife's tests use, and
//! for each one:
//!
//! 1. fetches the package from the Release the line lists, and refuses it unless
//!    `package_index::accept`, the installer's own decision, takes it under that line's SHA-256;
//! 2. writes it into a repository in basalt's layout under `target/basalt/`, whose index
//!    [`crate::disk`] hands to the test index host, so `script/swish-check` types `jig install
//!    ripgrep` against basalt's bytes;
//! 3. writes the package's program member to `target/basalt/programs/<triple>/<program>`, which the
//!    kernel harness's archive and the shell check's disk carry as `rg` for the lines that run it by
//!    path with hand-placed capabilities.
//!
//! It also fetches the host's reference `rg`, the same 14.1.1 from ripgrep's own Release, pinned by
//! SHA-256 per host ([`HOST_RIPGREP`]). `script/swish-check` compares the guest's walk against what
//! it prints, and `walk_pricing` holds it to the fixture's lines.
//!
//! **When [`crate::disk::BASALT_REPOSITORY`] names a repository**, as basalt's own `packages`
//! workflow does with the one it just built, nothing is fetched: that repository's packages are
//! accepted under its own index and their programs taken from it.
//!
//! **Without a network** a local run says what it could not fetch and succeeds, and every `rg`
//! line and test then skips and says why. In CI (`CI` set) a failed fetch fails the row.
//!
//! **Bumping the pin.** When basalt republishes and nife should take the new build, the lane that
//! wants it copies basalt's served lines over [`PINNED_INDEX`]'s file and lets CI judge them:
//!
//! ```text
//! curl -sSf https://basalt.nifeos.org/rolling/metadata/stand-in-index | grep '^ripgrep-' > xtask/basalt-pins.index
//! ```

use std::path::{Path, PathBuf};

use crate::host::workspace_root;

/// **basalt's index lines for the packages nife's tests install**, as basalt served them: one per
/// architecture, `<stem> sha256:<hex> <root>=<Release URL>`. The file is
/// `xtask/basalt-pins.index`; its name is provisional.
const PINNED_INDEX: &str = include_str!("../basalt-pins.index");

/// The channel basalt's repository is laid out under (`package_index::PROVISIONAL_CHANNEL`).
const CHANNEL: &str = "rolling";

/// **ripgrep 14.1.1 for the host, from its own Release** (`BurntSushi/ripgrep`), by host OS and
/// architecture: the asset's target triple and the SHA-256 its `.sha256` file published, read
/// 2026-10-13 (UTC). A host not listed has no reference `rg`, and the walk's expected answer falls
/// back to whatever `rg` is on `PATH`, whose version the staging line prints.
const HOST_RIPGREP: [(&str, &str, &str, &str); 4] = [
    (
        "macos",
        "aarch64",
        "aarch64-apple-darwin",
        "24ad76777745fbff131c8fbc466742b011f925bfa4fffa2ded6def23b5b937be",
    ),
    (
        "macos",
        "x86_64",
        "x86_64-apple-darwin",
        "fc87e78f7cb3fea12d69072e7ef3b21509754717b746368fd40d88963630e2b3",
    ),
    (
        "linux",
        "aarch64",
        "aarch64-unknown-linux-gnu",
        "c827481c4ff4ea10c9dc7a4022c8de5db34a5737cb74484d62eb94a95841ab2f",
    ),
    (
        "linux",
        "x86_64",
        "x86_64-unknown-linux-musl",
        "4cf9f2741e6c465ffdb7c26f38056a59e2a2544b51f7cc128ef28337eeae4d8e",
    ),
];

/// The ripgrep version [`HOST_RIPGREP`] names, which must be the one basalt's recipe builds.
const HOST_RIPGREP_VERSION: &str = "14.1.1";

/// **The repository the gates read**: the one [`crate::disk::BASALT_REPOSITORY`] names, or the
/// one `cargo xtask basalt-fetch` writes, `target/basalt`.
pub(crate) fn repository() -> PathBuf {
    std::env::var_os(crate::disk::BASALT_REPOSITORY)
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("target/basalt"))
}

/// The repository's index, which exists only once a fetch succeeded or basalt handed one over.
pub(crate) fn repository_index() -> PathBuf {
    repository().join(CHANNEL).join("metadata/stand-in-index")
}

/// **`rg` for `triple`, taken from basalt's package**: present only after `cargo xtask
/// basalt-fetch`. Never part of the base image (calef, 2026-10-07 UTC); the kernel harness's
/// archive and the shell check's disk carry it when it is here.
pub(crate) fn ripgrep_elf(triple: &str) -> PathBuf {
    workspace_root().join(format!("target/basalt/programs/{triple}/rg"))
}

/// **The host's reference `rg`**, ripgrep's own 14.1.1 build for this host, present only after
/// `cargo xtask basalt-fetch`. `walk_pricing` reads the same path.
pub(crate) fn host_ripgrep() -> PathBuf {
    workspace_root().join("target/ripgrep/host/rg")
}

/// `cargo xtask basalt-fetch`: fetch and check everything above. Returns whether the run passed.
pub(crate) fn fetch() -> bool {
    let in_ci = std::env::var_os("CI").is_some();
    let mut ok = true;
    let packages = match std::env::var_os(crate::disk::BASALT_REPOSITORY) {
        Some(_) => take_programs(&repository()),
        None => fetch_pinned().and_then(|()| take_programs(&repository())),
    };
    for result in [packages, fetch_host_ripgrep()] {
        match result {
            Ok(said) => println!("basalt-fetch: {said}"),
            Err(why) if in_ci => {
                eprintln!("basalt-fetch: {why}");
                ok = false;
            }
            Err(why) => eprintln!(
                "basalt-fetch: {why}; the `rg` lines and tests skip without it and say so"
            ),
        }
    }
    ok
}

/// Fetch every package [`PINNED_INDEX`] names into `target/basalt`, keeping a copy already there
/// that the index still accepts, and write the index beside them.
fn fetch_pinned() -> Result<(), String> {
    let index = package_index::Index::parse(PINNED_INDEX)
        .map_err(|e| format!("xtask/basalt-pins.index line {} is unreadable", e.number))?;
    let root = workspace_root().join("target/basalt").join(CHANNEL);
    let targets = root.join("targets");
    std::fs::create_dir_all(&targets).map_err(|e| format!("{}: {e}", targets.display()))?;
    for entry in index.entries() {
        let file = targets.join(format!("{}.nifepkg", entry.stem));
        let kept =
            std::fs::read(&file).is_ok_and(|bytes| package_index::accept(&entry, &bytes).is_ok());
        if kept {
            continue;
        }
        let location = entry
            .locations()
            .next()
            .ok_or_else(|| format!("{} lists no location to fetch it from", entry.stem))?;
        let url = format!(
            "https://{}:{}{}",
            location.host, location.port, location.path
        );
        let bytes = download(&url)?;
        package_index::accept(&entry, &bytes)
            .map_err(|why| format!("{url}: refused under its pinned digest ({why:?})"))?;
        std::fs::write(&file, &bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    let metadata = root.join("metadata");
    std::fs::create_dir_all(&metadata).map_err(|e| format!("{}: {e}", metadata.display()))?;
    std::fs::write(metadata.join("stand-in-index"), PINNED_INDEX)
        .map_err(|e| format!("{}: {e}", metadata.display()))
}

/// Accept each package `repository`'s index lists, by that index's digest, and write every program
/// it carries to [`ripgrep_elf`]'s directory for its architecture.
fn take_programs(repository: &Path) -> Result<String, String> {
    let index_path = repository.join(CHANNEL).join("metadata/stand-in-index");
    let text = std::fs::read_to_string(&index_path)
        .map_err(|e| format!("{}: {e}", index_path.display()))?;
    let index = package_index::Index::parse(&text)
        .map_err(|e| format!("{} line {} is unreadable", index_path.display(), e.number))?;
    let mut taken = Vec::new();
    for entry in index.entries() {
        let file = repository
            .join(CHANNEL)
            .join("targets")
            .join(format!("{}.nifepkg", entry.stem));
        let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let installable = package_index::accept(&entry, &bytes)
            .map_err(|why| format!("{}: refused by its index ({why:?})", file.display()))?;
        let architecture = entry.stem.rsplit('-').next().unwrap_or_default();
        let dir = workspace_root().join(format!(
            "target/basalt/programs/{architecture}-unknown-nife"
        ));
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for program in installable.programs() {
            std::fs::write(dir.join(program.name), program.bytes)
                .map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        taken.push(entry.stem);
    }
    Ok(format!(
        "{} accepted from {}",
        taken.join(", "),
        repository.display()
    ))
}

/// Fetch ripgrep's own build for this host into [`host_ripgrep`], unless a copy is there already.
fn fetch_host_ripgrep() -> Result<String, String> {
    let rg = host_ripgrep();
    if rg.exists() {
        return Ok(format!("host rg already at {}", rg.display()));
    }
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let (_, _, triple, want) = HOST_RIPGREP
        .iter()
        .find(|(o, a, _, _)| *o == os && *a == arch)
        .ok_or_else(|| format!("ripgrep publishes no {os} {arch} build this pins"))?;
    let name = format!("ripgrep-{HOST_RIPGREP_VERSION}-{triple}");
    let url = format!(
        "https://github.com/BurntSushi/ripgrep/releases/download/{HOST_RIPGREP_VERSION}/{name}.tar.gz"
    );
    let bytes = download(&url)?;
    let got = measured_boot::digest_text(&package_archive::sha256(&bytes));
    if got[b"sha256:".len()..] != *want.as_bytes() {
        return Err(format!("{url}: SHA-256 is not the pinned {want}"));
    }
    let dir = rg.parent().expect("host_ripgrep has a parent");
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tarball = dir.join(format!("{name}.tar.gz"));
    std::fs::write(&tarball, &bytes).map_err(|e| format!("{}: {e}", tarball.display()))?;
    let unpacked = std::process::Command::new("tar")
        .arg("xzf")
        .arg(&tarball)
        .args(["--strip-components=1", "-C"])
        .arg(dir)
        .arg(format!("{name}/rg"))
        .status()
        .is_ok_and(|s| s.success());
    let _ = std::fs::remove_file(&tarball);
    if !unpacked || !rg.exists() {
        return Err(format!("could not unpack rg from {url}"));
    }
    Ok(format!(
        "host rg {HOST_RIPGREP_VERSION} at {}",
        rg.display()
    ))
}

/// The bytes at `url`, by `curl`, following redirects (a Release asset answers with one). Nothing
/// fetched is trusted until its caller checks a pinned digest, so the transport only has to arrive.
fn download(url: &str) -> Result<Vec<u8>, String> {
    let out = std::process::Command::new("curl")
        .args(["-sSfL", "--retry", "3", "--max-time", "120", url])
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "could not fetch {url}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}
