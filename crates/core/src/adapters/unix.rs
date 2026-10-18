//! Platform adapter outside Windows: hardlinks from the file metadata; no detection of
//! files in use yet (ADR-0022).

use crate::ports::{FileIdentity, Platform};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct UnixPlatform;

impl Platform for UnixPlatform {
    fn file_identity(&self, path: &Path) -> io::Result<FileIdentity> {
        let metadata = fs::symlink_metadata(path)?;
        Ok(FileIdentity {
            volume: metadata.dev(),
            index: metadata.ino(),
            links: u32::try_from(metadata.nlink()).unwrap_or(u32::MAX),
        })
    }

    fn delete_blocked(&self, _path: &Path) -> bool {
        false
    }

    fn process_using(&self, _path: &Path) -> Option<String> {
        None
    }
}
