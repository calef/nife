//! **The modules generated into std verbatim** from host-tested crates, and the doc-example strip
//! and module inlining that copy needs.
//!
//! Cut out of `farm.rs` unchanged on 2026-10-12 (UTC) by lane maintainer/near-ceiling-splits, for
//! §266 (a Rust source file stays under 2,000 lines) and §282 (when a file is split, the parent
//! becomes an index). What was private to `farm` is `pub(super)` here, which reaches the same
//! modules.
//!
//! Name: provisional (lane maintainer/near-ceiling-splits, 2026-10-12 UTC), for the module and the
//! file.

use super::*;

/// Remove `# Examples` sections from the doc comments of a file about to be copied into the patched
/// std sysroot.
///
/// A doctest in one of these crates says `use entropy_contract::...`, and in the copy there is no such
/// crate: the file arrives as `sys/pal/nife/entropyproto.rs`, an inner module of `std`. So the
/// example is *false* in its destination, in the specific way milestone 68 cares about, which is
/// that it teaches a reader of the PAL something that is not true of the code they are reading.
/// Nothing runs std's doctests here, so this is a documentation fix rather than a build fix; it is
/// done at the copy because the alternative is refusing the workspace crates real examples, and the
/// workspace is where the example is checked.
///
/// Prose and `text` blocks survive: this drops a `# Examples` heading and everything under it, up to
/// the next heading at the same level or the end of the doc block. Fence state is tracked, so a
/// hidden doctest line (`# use ...`) inside a code block is not mistaken for that next heading.
pub(super) fn strip_doc_examples(body: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut skipping = false;
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let Some(content) = trimmed
            .strip_prefix("//!")
            .or_else(|| trimmed.strip_prefix("///"))
        else {
            // Any non-doc line ends the doc block, and with it the section being skipped.
            skipping = false;
            in_fence = false;
            out.push(line);
            continue;
        };
        let content = content.trim();
        if skipping {
            if content.starts_with("```") {
                in_fence = !in_fence;
            } else if !in_fence && content.starts_with("# ") {
                skipping = false;
                out.push(line);
            }
            continue;
        }
        if content == "# Examples" || content == "# Example" {
            skipping = true;
            continue;
        }
        out.push(line);
    }
    out.join("\n")
}

/// Generate `abi.rs` and `user_mode_heap.rs` verbatim from the host-tested crates, so the ABI numbers and
/// the heap algorithm have exactly one definition. The transform strips crate-level inner
/// attributes (`#![no_std]`, illegal in a non-root module), any trailing `#[cfg(test)]` module, and
/// any `# Examples` section (see [`strip_doc_examples`] for why that last one).
pub(super) fn std_generate_modules() -> bool {
    let root = workspace_root();
    let jobs = [
        (
            root.join("crates/abi/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/abi.rs"),
        ),
        (
            root.join("crates/user_mode_heap/src/lib.rs"),
            farm_std_src().join("sys/alloc/nife/user_mode_heap.rs"),
        ),
        // Each thread's cache in front of that heap (milestone 561 (a per-CPU allocator is what
        // the current-CPU page was for)), a sibling of it, so the two stay one grid.
        (
            root.join("crates/allocation_cache/src/lib.rs"),
            farm_std_src().join("sys/alloc/nife/allocation_cache.rs"),
        ),
        // The net_stack socket-contract wire format, verbatim, so the net PAL cannot drift from the
        // server it talks to (same discipline as the ABI and heap crates above).
        (
            root.join("crates/socket_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/netproto.rs"),
        ),
        // The FS-service wire protocol (DECISIONS §27), so `std::fs`'s PAL cannot drift from the
        // server it opens files through. Same discipline as the three above.
        (
            root.join("crates/filesystem_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/fsproto.rs"),
        ),
        // The wall-clock contract (DECISIONS §43), so the time PAL reads the clock page with the
        // same seqlock and the same layout the clock service publishes it with. Same discipline as
        // the four above; this one matters more than most, because a drift here would be a torn
        // read of a timestamp rather than a compile error.
        (
            root.join("crates/clock_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/clockproto.rs"),
        ),
        // The entropy contract (DECISIONS §44), so the random PAL packs its requests and reads its
        // replies exactly the way the entropy service serves them. Same discipline as the five
        // above; a drift here would be a program reading the wrong bytes as a key.
        (
            root.join("crates/entropy_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/entropyproto.rs"),
        ),
        // The inert-configuration contract (milestone 47's environment-variable fork, DECISIONS
        // §111), so `sys/env`'s seeding reads the config page with the same layout and the same
        // validated domains whoever assembles a page uses. Same discipline as the six above.
        (
            root.join("crates/environment_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/envproto.rs"),
        ),
        // The argument page (milestone 205 (how a foreign program is told what to do), DECISIONS
        // §170), so `sys/args` reads the argv with the layout the shell assembles it in.
        (
            root.join("crates/program_argument_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/argproto.rs"),
        ),
        // The byte-sink contract (milestone 50), so `println!`'s framing and the classification of
        // a failed SEND are one definition shared with every sink and with the kernel-side tests.
        // Same discipline as the six above, and the one that would hurt most to get wrong: a drift
        // in `GONE` would be a program that keeps printing into a pipe whose reader has exited.
        (
            root.join("crates/byte_sink_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/sinkproto.rs"),
        ),
        // The std runtime contract (milestone 595 (provisional)): the eight fixed slots and the three
        // shared pages, which `rt` re-exports. The progenitor places a std child's capabilities by
        // these numbers, so a drift here would be a program reading its heap budget out of the
        // slot its stdout went in.
        (
            root.join("crates/std_runtime_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/runtimeproto.rs"),
        ),
        // The name resolver's client wire (milestone 384 (in a capability system the resolver is a
        // grant)), so `lookup_host` asks the resolver with the words it serves. Only `wire.rs`: the
        // rest of that crate needs `domain_name_system`, which the PAL cannot name. Milestone 801
        // (packages over the internet).
        (
            root.join("crates/domain_name_resolution_contract/src/wire.rs"),
            farm_std_src().join("sys/pal/nife/resolveproto.rs"),
        ),
        // The timebase page (milestone 184), so `rt::cntfrq` reads the machine's rate at the
        // address and with the magic the kernel writes it with. Generated for every farm and
        // compiled only on x86_64 and riscv64 (`sys/pal/nife/mod.rs` gates the module), because the
        // farm is one source tree for all three targets and aarch64 reads a register instead.
        (
            root.join("crates/counter_frequency_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/counterfreqproto.rs"),
        ),
        // The current-CPU page (milestone 812): a thread's allowance, which is
        // `available_parallelism`'s answer, at the layout the kernel builds it with. It names one
        // constant of the address-space map, so the map comes too, and the copy below points the
        // name at it.
        (
            root.join("crates/address_space_map/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/addressmap.rs"),
        ),
        (
            root.join("crates/current_cpu_contract/src/lib.rs"),
            farm_std_src().join("sys/pal/nife/currentcpuproto.rs"),
        ),
    ];
    for (src, dst) in jobs {
        let Ok(text) = std::fs::read_to_string(&src) else {
            eprintln!("std-src: cannot read {}", src.display());
            return false;
        };
        let mut body: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("#!["))
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(idx) = body.find("\n#[cfg(test)]\nmod tests") {
            body.truncate(idx);
        }
        let modules = out_of_line_modules(&src, &body);
        let body = match inline_modules(&body, |name| {
            let (_, path) = modules.iter().find(|(n, _)| n == name)?;
            std::fs::read_to_string(path).ok()
        }) {
            Ok(body) => body,
            Err(e) => {
                eprintln!("std-src: {}: {e}", src.display());
                return false;
            }
        };
        let body = format!("{}\n", strip_doc_examples(&body).trim_end());
        if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // The one generated file that names another workspace crate: inside std the map is a
        // sibling module, not a crate.
        let body = body.replace("address_space_map::", "super::addressmap::");
        if let Err(e) = std::fs::write(&dst, body) {
            eprintln!("std-src: cannot write {}: {e}", dst.display());
            return false;
        }
    }
    true
}

/// The modules a crate root names out of line, as a column-0 `pub mod x;`, each with the `x.rs`
/// beside the root that holds it. Only that form: a `mod x;` or a `#[cfg(test)] mod tests;` is not
/// public, so it is nothing std copies.
pub(super) fn out_of_line_modules(root_file: &Path, text: &str) -> Vec<(String, PathBuf)> {
    let dir = root_file.parent().unwrap_or(Path::new("."));
    text.lines()
        .filter_map(|l| l.strip_prefix("pub mod ")?.strip_suffix(';'))
        .filter(|name| {
            !name.is_empty() && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
        })
        .map(|name| (name.to_string(), dir.join(format!("{name}.rs"))))
        .collect()
}

/// Put each column-0 `pub mod x;` in `body` back inline, as `pub mod x {`, the text `read(x)`
/// returns indented four spaces, and `}`, so a crate split into module files still generates the
/// one file std builds (milestone 845, §274). An empty line stays empty, so nothing gains
/// trailing whitespace. A module file may end with `#[cfg(test)] mod tests;`: it is copied with
/// the rest, and std never builds with `cfg(test)`, so the file it names is never looked for.
/// An out-of-line module inside a module file is not followed, so it fails the std build loudly
/// rather than being skipped.
pub(super) fn inline_modules(
    body: &str,
    read: impl Fn(&str) -> Option<String>,
) -> Result<String, String> {
    let names = out_of_line_modules(Path::new("lib.rs"), body);
    let mut out: Vec<String> = Vec::new();
    for line in body.lines() {
        let name = line
            .strip_prefix("pub mod ")
            .and_then(|l| l.strip_suffix(';'))
            .filter(|n| names.iter().any(|(m, _)| m == n));
        let Some(name) = name else {
            out.push(line.to_string());
            continue;
        };
        let Some(text) = read(name) else {
            return Err(format!("cannot read the file of `pub mod {name};`"));
        };
        out.push(format!("pub mod {name} {{"));
        for l in text.trim_end().lines() {
            out.push(if l.is_empty() {
                String::new()
            } else {
                format!("    {l}")
            });
        }
        out.push("}".to_string());
    }
    Ok(out.join("\n"))
}
