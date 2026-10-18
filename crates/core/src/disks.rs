//! Used and free space of the drives that hold the analyzed folders (T-08.01).

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveSpace {
    /// Drive root, e.g. `C:\`.
    pub root: PathBuf,
    pub total: u64,
    pub available: u64,
}

impl DriveSpace {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.available)
    }

    /// Free space in percent (0–100), for the "disco cheio" alert (T-10.04).
    pub fn free_percent(&self) -> u64 {
        (self.available * 100).checked_div(self.total).unwrap_or(0)
    }
}

/// Root of the drive holding `path` (`C:\` for `C:\Projetos\api`); `None` for relative paths.
pub fn drive_root(path: &Path) -> Option<PathBuf> {
    let mut root = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => root.push(component),
            _ => break,
        }
    }
    root.has_root().then_some(root)
}

/// One entry per distinct drive among `paths`, skipping drives that cannot be read
/// (an unplugged disk is not an error).
pub fn drives(paths: &[PathBuf]) -> Vec<DriveSpace> {
    let mut roots = BTreeMap::new();
    for path in paths {
        if let Some(root) = drive_root(path) {
            roots.insert(root.to_string_lossy().to_lowercase(), root);
        }
    }
    roots
        .into_values()
        .filter_map(|root| {
            let total = fs4::total_space(&root).ok()?;
            let available = fs4::available_space(&root).ok()?;
            Some(DriveSpace {
                root,
                total,
                available,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_root_is_the_prefix_and_root_of_the_path() {
        if cfg!(windows) {
            assert_eq!(
                drive_root(Path::new("C:\\Projetos\\api")),
                Some(PathBuf::from("C:\\"))
            );
        }
        assert_eq!(drive_root(Path::new("relativo\\pasta")), None);
    }

    #[test]
    fn each_drive_appears_once_with_real_numbers() {
        let here = std::env::current_dir().unwrap();
        let drives = drives(&[here.clone(), here.join("outra"), here.join("x")]);
        assert_eq!(drives.len(), 1);
        let drive = &drives[0];
        assert!(drive.total > 0);
        assert!(drive.available <= drive.total);
        assert_eq!(drive.used(), drive.total - drive.available);
        assert!(drive.free_percent() <= 100);
    }

    #[test]
    fn unreadable_drives_are_skipped() {
        let missing = if cfg!(windows) {
            "Q:\\nao-existe"
        } else {
            "/nao-existe-xyz"
        };
        let drives = drives(&[PathBuf::from(missing)]);
        assert!(drives.iter().all(|d| d.total > 0));
    }

    #[test]
    fn percentages_handle_empty_and_full_drives() {
        let empty = DriveSpace {
            root: "C:\\".into(),
            total: 0,
            available: 0,
        };
        assert_eq!(empty.free_percent(), 0);
        let quarter = DriveSpace {
            root: "C:\\".into(),
            total: 400,
            available: 100,
        };
        assert_eq!((quarter.free_percent(), quarter.used()), (25, 300));
        let odd = DriveSpace {
            root: "C:\\".into(),
            total: 3,
            available: 5,
        };
        assert_eq!(odd.used(), 0);
    }
}
