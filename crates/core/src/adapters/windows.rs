//! Platform adapter for Windows (ADR-0022): hardlinks through `GetFileInformationByHandle`
//! and files in use through a delete-access open plus the Restart Manager.
//! The only module of the core with `unsafe`; every block says why it is sound.
#![allow(unsafe_code)]

use crate::ports::{FileIdentity, Platform};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SHARING_VIOLATION};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, DELETE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    GetFileInformationByHandle,
};
use windows_sys::Win32::System::RestartManager::{
    CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
    RmStartSession,
};

/// Opens files and folders without following links and without blocking anyone.
const SHARE_ALL: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
const NO_FOLLOW: u32 = FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS;

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPlatform;

fn open(path: &Path, access: u32) -> io::Result<File> {
    OpenOptions::new()
        .access_mode(access)
        .share_mode(SHARE_ALL)
        .custom_flags(NO_FOLLOW)
        .open(path)
}

impl Platform for WindowsPlatform {
    fn file_identity(&self, path: &Path) -> io::Result<FileIdentity> {
        let file = open(path, FILE_READ_ATTRIBUTES)?;
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: the handle is valid while `file` lives, and `info` is a writable struct
        // of the type the function fills.
        let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &raw mut info) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(FileIdentity {
            volume: u64::from(info.dwVolumeSerialNumber),
            index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
            links: info.nNumberOfLinks,
        })
    }

    fn delete_blocked(&self, path: &Path) -> bool {
        // Asking for delete access fails when another process opened the file without
        // allowing deletion, which is exactly when removing it would fail.
        matches!(
            open(path, DELETE),
            Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION as i32)
        )
    }

    fn process_using(&self, path: &Path) -> Option<String> {
        let session = RestartManagerSession::start()?;
        session.processes_using(path)?.into_iter().next()
    }
}

/// A Restart Manager session, ended when dropped.
struct RestartManagerSession(u32);

impl RestartManagerSession {
    fn start() -> Option<Self> {
        let mut handle = 0u32;
        let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
        // SAFETY: both pointers are valid and `key` has the documented minimum length.
        let status = unsafe { RmStartSession(&raw mut handle, 0, key.as_mut_ptr()) };
        (status == 0).then_some(Self(handle))
    }

    fn processes_using(&self, path: &Path) -> Option<Vec<String>> {
        let name: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let names = [name.as_ptr()];
        // SAFETY: one NUL-terminated file name, valid during the call; no apps or services.
        let status = unsafe {
            RmRegisterResources(self.0, 1, names.as_ptr(), 0, ptr::null(), 0, ptr::null())
        };
        if status != 0 {
            return None;
        }
        let mut capacity = 4u32;
        loop {
            let mut infos = vec![RM_PROCESS_INFO::default(); capacity as usize];
            let (mut needed, mut count, mut reasons) = (0u32, capacity, 0u32);
            // SAFETY: `infos` holds `count` writable entries; the other pointers are valid.
            let status = unsafe {
                RmGetList(
                    self.0,
                    &raw mut needed,
                    &raw mut count,
                    infos.as_mut_ptr(),
                    &raw mut reasons,
                )
            };
            match status {
                0 => {
                    return Some(
                        infos
                            .iter()
                            .take(count as usize)
                            .map(|info| app_name(&info.strAppName))
                            .filter(|name| !name.is_empty())
                            .collect(),
                    );
                }
                // The list grew between calls: ask again with room for everyone.
                ERROR_MORE_DATA if needed > capacity => capacity = needed,
                _ => return None,
            }
        }
    }
}

impl Drop for RestartManagerSession {
    fn drop(&mut self) {
        // SAFETY: the session was started by `start` and is ended exactly once.
        unsafe {
            RmEndSession(self.0);
        }
    }
}

fn app_name(raw: &[u16]) -> String {
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use limpador_testkit::LockedFile;
    use std::fs;

    #[test]
    fn hardlinks_share_the_identity_and_count_their_names() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("store.js");
        let link = dir.path().join("node_modules.js");
        fs::write(&original, "x").unwrap();
        fs::hard_link(&original, &link).unwrap();
        let alone = dir.path().join("sozinho.js");
        fs::write(&alone, "y").unwrap();

        let a = WindowsPlatform.file_identity(&original).unwrap();
        let b = WindowsPlatform.file_identity(&link).unwrap();
        let c = WindowsPlatform.file_identity(&alone).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.links, 2);
        assert_eq!(c.links, 1);
        assert_ne!(a.index, c.index);
        assert!(
            WindowsPlatform
                .file_identity(&dir.path().join("nada"))
                .is_err()
        );
    }

    #[test]
    fn a_file_held_open_blocks_deletion_and_names_its_process() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("em uso.js");
        fs::write(&file, "x").unwrap();
        assert!(!WindowsPlatform.delete_blocked(&file));
        assert_eq!(WindowsPlatform.process_using(&file), None);

        let _held = LockedFile::open(&file).unwrap();
        assert!(WindowsPlatform.delete_blocked(&file));
        let process = WindowsPlatform
            .process_using(&file)
            .expect("this test process");
        assert!(!process.is_empty());
        assert!(!WindowsPlatform.delete_blocked(&dir.path().join("nada")));
    }

    #[test]
    fn app_names_stop_at_the_terminator() {
        let mut raw = [0u16; 8];
        raw[..4].copy_from_slice(&[u16::from(b'n'), u16::from(b'o'), u16::from(b'd'), 0]);
        assert_eq!(app_name(&raw), "nod");
        assert_eq!(app_name(&[u16::from(b'x'); 3]), "xxx");
    }
}
