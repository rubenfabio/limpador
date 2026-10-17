use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use std::process::Command;

/// Creates a directory junction (Windows) or a symlink (other systems) at `link`
/// pointing to `target`. Junctions need no administrator rights.
pub fn create_junction(link: &Path, target: &Path) -> io::Result<()> {
    if cfg!(windows) {
        let status = Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .output()?;
        if !status.status.success() {
            return Err(io::Error::other(format!(
                "mklink /J failed: {}",
                String::from_utf8_lossy(&status.stderr)
            )));
        }
        Ok(())
    } else {
        symlink_dir(target, link)
    }
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn symlink_dir(_target: &Path, _link: &Path) -> io::Result<()> {
    unreachable!("junctions are used on Windows")
}

/// Keeps a file open so other processes see it as "in use" until dropped.
/// On Windows the file is opened without sharing, so deletes and writes fail.
#[derive(Debug)]
pub struct LockedFile {
    _file: File,
}

impl LockedFile {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        Ok(Self {
            _file: options.open(path)?,
        })
    }
}
