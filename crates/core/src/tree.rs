//! Folders on disk, walked without ever following links or junctions (I3): sizes that
//! respect hardlinks, files in use, and removal.

use crate::collector::StopSignal;
use crate::ports::Platform;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeSize {
    pub apparent: u64,
    /// Bytes that come back to the disk if every folder is removed.
    pub freeable: u64,
}

/// Calls `visit` for every regular file under `folder`. Links and junctions are neither
/// followed nor visited. Stops early (returning `false`) when `visit` returns `false` or
/// `stop` fires.
fn walk_files(folder: &Path, stop: &StopSignal, visit: &mut dyn FnMut(&Path, u64) -> bool) -> bool {
    let mut pending = vec![folder.to_path_buf()];
    while let Some(dir) = pending.pop() {
        if stop.should_stop() {
            return false;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                if !visit(&entry.path(), size) {
                    return false;
                }
            }
        }
    }
    true
}

/// Sizes of `folders` together. A file with several hardlinks only counts as freeable
/// when all its names are inside these folders: otherwise its bytes stay on disk
/// (the pnpm store, T-04.01).
pub fn measure(folders: &[PathBuf], platform: &dyn Platform, stop: &StopSignal) -> TreeSize {
    let mut size = TreeSize::default();
    // (volume, index) → (bytes, names on disk, names seen here)
    let mut linked: HashMap<(u64, u64), (u64, u32, u32)> = HashMap::new();
    for folder in folders {
        walk_files(folder, stop, &mut |path, bytes| {
            size.apparent += bytes;
            match platform.file_identity(path) {
                Ok(id) if id.links > 1 => {
                    linked
                        .entry((id.volume, id.index))
                        .or_insert((bytes, id.links, 0))
                        .2 += 1;
                }
                _ => size.freeable += bytes,
            }
            true
        });
    }
    size.freeable += linked
        .values()
        .filter(|(_, links, seen)| seen >= links)
        .map(|(bytes, _, _)| bytes)
        .sum::<u64>();
    size
}

/// First file under `folders` that another process keeps from being deleted (I5).
pub fn first_in_use(
    folders: &[PathBuf],
    platform: &dyn Platform,
    stop: &StopSignal,
) -> Option<PathBuf> {
    let mut found = None;
    for folder in folders {
        walk_files(folder, stop, &mut |path, _| {
            if platform.delete_blocked(path) {
                found = Some(path.to_path_buf());
                return false;
            }
            true
        });
        if found.is_some() {
            break;
        }
    }
    found
}

/// Removes `folder` and everything in it. A link or junction (the folder itself or
/// anything inside) is removed as a link: its target is never touched (I3).
pub fn remove(folder: &Path) -> io::Result<()> {
    let kind = fs::symlink_metadata(folder)?.file_type();
    if kind.is_symlink() {
        // A junction or directory symlink is removed like an empty folder on Windows.
        return fs::remove_dir(folder).or_else(|_| fs::remove_file(folder));
    }
    // The standard library removes links inside as links, without following them.
    fs::remove_dir_all(folder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::native_platform;
    use limpador_testkit::{LockedFile, create_junction};

    fn never() -> StopSignal {
        StopSignal::new(None)
    }

    fn file(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![b'x'; bytes]).unwrap();
    }

    #[test]
    fn hardlinks_shared_with_the_outside_are_not_freeable() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        let modules = dir.path().join("projeto").join("node_modules");
        file(&store.join("react.js"), 1_000);
        file(&modules.join("proprio.js"), 300);
        fs::create_dir_all(modules.join("react")).unwrap();
        fs::hard_link(
            store.join("react.js"),
            modules.join("react").join("index.js"),
        )
        .unwrap();
        // Two names, both inside the item: freed when it goes.
        file(&modules.join("a.js"), 50);
        fs::hard_link(modules.join("a.js"), modules.join("b.js")).unwrap();

        let size = measure(
            std::slice::from_ref(&modules),
            native_platform().as_ref(),
            &never(),
        );

        assert_eq!(size.apparent, 1_000 + 300 + 50 + 50);
        assert_eq!(size.freeable, 300 + 50);
    }

    #[test]
    fn junctions_are_not_followed_when_measuring_or_removing() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("pacote-de-fora");
        file(&outside.join("grande.bin"), 5_000);
        let modules = dir.path().join("node_modules");
        file(&modules.join("index.js"), 10);
        create_junction(&modules.join("link"), &outside).unwrap();

        let size = measure(
            std::slice::from_ref(&modules),
            native_platform().as_ref(),
            &never(),
        );
        assert_eq!(
            size,
            TreeSize {
                apparent: 10,
                freeable: 10
            }
        );

        remove(&modules).unwrap();
        assert!(!modules.exists());
        assert!(
            outside.join("grande.bin").exists(),
            "the junction's target stays"
        );
    }

    #[test]
    fn removing_a_junction_removes_only_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("alvo");
        file(&target.join("dado.txt"), 3);
        let link = dir.path().join("link");
        create_junction(&link, &target).unwrap();

        remove(&link).unwrap();
        assert!(!link.exists());
        assert!(target.join("dado.txt").exists());
    }

    #[test]
    fn long_paths_and_accented_names_are_measured_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("node_modules");
        let mut deep = modules.clone();
        while deep.as_os_str().len() < 300 {
            deep.push("pacote com espaço e acentuação");
        }
        file(&deep.join("índice.js"), 7);

        let size = measure(
            std::slice::from_ref(&modules),
            native_platform().as_ref(),
            &never(),
        );
        assert_eq!(size.apparent, 7);
        remove(&modules).unwrap();
        assert!(!modules.exists());
    }

    #[test]
    fn read_only_files_are_removed_too() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("node_modules");
        file(&modules.join("travado.js"), 1);
        let mut permissions = fs::metadata(modules.join("travado.js"))
            .unwrap()
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(modules.join("travado.js"), permissions).unwrap();

        remove(&modules).unwrap();
        assert!(!modules.exists());
    }

    #[test]
    fn the_first_file_in_use_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("node_modules");
        file(&modules.join("livre.js"), 1);
        file(&modules.join("vite").join("em-uso.js"), 1);
        let platform = native_platform();
        let folders = vec![modules.clone()];
        assert_eq!(first_in_use(&folders, platform.as_ref(), &never()), None);

        let _held = LockedFile::open(&modules.join("vite").join("em-uso.js")).unwrap();
        if cfg!(windows) {
            assert_eq!(
                first_in_use(&folders, platform.as_ref(), &never()),
                Some(modules.join("vite").join("em-uso.js"))
            );
        }
    }

    #[test]
    fn a_stopped_walk_returns_what_it_had() {
        let dir = tempfile::tempdir().unwrap();
        file(&dir.path().join("a.js"), 5);
        let stopped = StopSignal::new(None);
        stopped.cancel();
        let size = measure(
            &[dir.path().to_path_buf()],
            native_platform().as_ref(),
            &stopped,
        );
        assert_eq!(size, TreeSize::default());
    }
}
