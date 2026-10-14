//! **The real machine**: `Host` for each operating system the program runs on, over that
//! system's own module, and a refusal everywhere else.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// This machine's disks.
pub struct Machine;

#[cfg(target_os = "macos")]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        macos::discover()
    }

    fn can_erase(&self, _disk: &Disk) -> Result<(), String> {
        Ok(())
    }

    /// `diskutil eraseDisk`, then find the new volume where macOS mounted it, mounting it if the
    /// disk was attached without mounting (as a file-backed disk from `hdiutil -nomount` is).
    fn erase(&self, disk: &Disk) -> Result<PathBuf, String> {
        macos::erase(disk)?;
        for _ in 0..20 {
            let disks = macos::discover()?;
            if let Some(found) = disks.iter().find(|d| d.id == disk.id)
                && let Some(v) = found
                    .volumes
                    .iter()
                    .find(|v| v.filesystem == crate::disk::Filesystem::Fat)
            {
                if let Some(mount) = &v.mount {
                    return Ok(mount.clone());
                }
                macos::mount(&v.id)?;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        Err(format!(
            "{} was erased but its new volume never appeared mounted",
            disk.id
        ))
    }

    fn finish(&self, disk: &Disk, _root: &Path) -> Result<String, String> {
        macos::eject(disk)?;
        Ok(format!("Ejected {}; it can be pulled out.", disk.id))
    }
}

#[cfg(target_os = "linux")]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        linux::discover()
    }

    fn can_erase(&self, _disk: &Disk) -> Result<(), String> {
        Ok(())
    }

    fn erase(&self, disk: &Disk) -> Result<PathBuf, String> {
        let partition = linux::erase(disk)?;
        linux::mount(&partition)
    }

    fn finish(&self, _disk: &Disk, root: &Path) -> Result<String, String> {
        linux::unmount(root)?;
        Ok(format!(
            "Unmounted {}; it can be pulled out.",
            root.display()
        ))
    }
}

#[cfg(windows)]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        windows::discover()
    }

    /// Not automated on Windows; see `crate::windows`'s BUGS. The steps by hand are the answer.
    fn can_erase(&self, disk: &Disk) -> Result<(), String> {
        Err(crate::windows::erase_instructions(disk))
    }

    fn erase(&self, disk: &Disk) -> Result<PathBuf, String> {
        Err(crate::windows::erase_instructions(disk))
    }

    fn finish(&self, disk: &Disk, _root: &Path) -> Result<String, String> {
        Ok(format!(
            "Use \"Safely Remove Hardware\" on {} before pulling it out.",
            disk.id
        ))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
impl Host for Machine {
    fn discover(&self) -> Result<Vec<Disk>, String> {
        Err("this program does not know how to find disks on this operating system".to_owned())
    }

    fn can_erase(&self, _disk: &Disk) -> Result<(), String> {
        Err("erasing is not supported on this operating system".to_owned())
    }

    fn erase(&self, _disk: &Disk) -> Result<PathBuf, String> {
        Err("erasing is not supported on this operating system".to_owned())
    }

    fn finish(&self, _disk: &Disk, _root: &Path) -> Result<String, String> {
        Ok(String::new())
    }
}
