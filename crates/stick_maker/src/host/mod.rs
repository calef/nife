//! **The half that runs the host's own tools**: `diskutil` on macOS, `sfdisk`, `mkfs.vfat` and
//! `mount` on Linux, four `kernel32` calls on Windows; and the real terminal.
//!
//! Everything that decides anything is elsewhere and tested: which disks may be written
//! (`crate::disk`), what each host's description of a disk means (`crate::macos`, `crate::linux`,
//! `crate::windows`), and the whole conversation with the person (`crate::cli`, against a fake
//! host). What is left here only runs a program or makes a system call and reports what came back.
//!
//! **Why this directory is outside `script/coverage`'s floor, recorded there too**: the coverage
//! run is an unprivileged Linux process, and nothing here can run in it. There is no `diskutil`
//! on Linux and no `kernel32`; the Linux arm erases a whole block device as root. Each arm is
//! instead run for real on its own host: macOS by `helpers/stick-maker-proof.sh` against
//! hdiutil-attached files, Linux and Windows by `.github/workflows/stick-maker-hosts.yml` (a loop
//! device through both paths; Windows discovery). That is a stronger signal than a line count, and
//! it is where a regression here would show.

use std::io::{self, BufRead as _, Write as _};
use std::path::{Path, PathBuf};

use crate::cli::{Console, Host};
use crate::disk::Disk;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

mod machine;
mod terminal;

pub use machine::*;
pub use terminal::*;
