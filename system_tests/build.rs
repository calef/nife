//! The system-test image links with the kernel's own linker script, and declares
//! `cfg(initial_ramdisk)`.
//!
//! `cargo::rustc-link-arg` is per package: the kernel's `build.rs` passes `-T link-<arch>.ld` for
//! the kernel's own targets and for nobody who depends on it. This image is the same kernel, so it
//! has to ask for the same layout itself. `cfg(initial_ramdisk)` is declared the way
//! `kernel/build.rs` declares it (`declare_initial_ramdisk_cfg`), because the suite's modules are
//! gated on it and a build script's `rustc-cfg` reaches only its own package too.

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let link_script = match arch.as_str() {
        "aarch64" => "link-aarch64.ld",
        "riscv64" => "link-riscv64.ld",
        "x86_64" => "link-x86_64.ld",
        other => panic!("system_tests: no kernel linker script for {other}"),
    };
    println!("cargo::rerun-if-changed=../kernel/{link_script}");
    println!("cargo::rustc-link-arg=-T{manifest_dir}/../kernel/{link_script}");
    // **Whether a missing `rg` fails rather than skips** (milestone 121 (`ripgrep` on nife), lane
    // milestone/121-rg-threads): `ripgrep_threads_tests` reads it with `env!`. The CI rows that
    // build `rg` set it to 1, so a build that left `rg` out of the archive cannot pass as a skip.
    println!("cargo::rerun-if-env-changed=NIFE_RIPGREP_REQUIRED");
    println!(
        "cargo::rustc-env=NIFE_RIPGREP_REQUIRED={}",
        std::env::var("NIFE_RIPGREP_REQUIRED").unwrap_or_default()
    );
    // **And a missing `ioping`** (milestone 906 (ioping is packaged in basalt, and its test
    // installs it with `jig`)), on the same terms: `c_program_tests` reads it, and the CI rows that
    // fetch basalt's ioping set it to 1. Name provisional.
    println!("cargo::rerun-if-env-changed=NIFE_IOPING_REQUIRED");
    println!(
        "cargo::rustc-env=NIFE_IOPING_REQUIRED={}",
        std::env::var("NIFE_IOPING_REQUIRED").unwrap_or_default()
    );
    // **And a missing `hackbench`** (milestone 907 (hackbench is packaged in basalt)), the same
    // way, for `c_thread_tests`. Name provisional.
    println!("cargo::rerun-if-env-changed=NIFE_HACKBENCH_REQUIRED");
    println!(
        "cargo::rustc-env=NIFE_HACKBENCH_REQUIRED={}",
        std::env::var("NIFE_HACKBENCH_REQUIRED").unwrap_or_default()
    );
    println!("cargo::rustc-check-cfg=cfg(initial_ramdisk)");
    if matches!(arch.as_str(), "aarch64" | "riscv64" | "x86_64") {
        println!("cargo::rustc-cfg=initial_ramdisk");
    }
}
