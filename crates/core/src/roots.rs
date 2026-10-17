//! Root folders and exclusions (T-02.02). Nothing outside the roots is analyzed (I4).

use crate::config::Roots;
use crate::glob;
use crate::message::Message;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddRoot {
    Added,
    /// Already inside another root: nothing changes.
    AlreadyCovered {
        by: PathBuf,
    },
    /// The new root contains existing ones, which it replaces.
    Replaced {
        removed: Vec<PathBuf>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootError {
    NotFound,
    NotReadable,
}

/// Comparable spelling of a path: `/` separators, no trailing `/`, and lower case on
/// Windows, whose file system ignores case.
pub fn path_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.trim_end_matches('/');
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.to_owned()
    }
}

/// Whether `inner` is `outer` itself or somewhere inside it.
pub fn contains(outer: &Path, inner: &Path) -> bool {
    let (outer, inner) = (path_key(outer), path_key(inner));
    inner == outer || inner.starts_with(&format!("{outer}/"))
}

pub fn add_root(roots: &mut Roots, path: &Path) -> Result<AddRoot, RootError> {
    match fs::read_dir(path) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(RootError::NotFound),
        Err(_) => return Err(RootError::NotReadable),
    }
    if let Some(by) = roots.paths.iter().find(|root| contains(root, path)) {
        return Ok(AddRoot::AlreadyCovered { by: by.clone() });
    }
    let (removed, kept): (Vec<PathBuf>, Vec<PathBuf>) =
        roots.paths.drain(..).partition(|root| contains(path, root));
    roots.paths = kept;
    roots.paths.push(path.to_owned());
    Ok(if removed.is_empty() {
        AddRoot::Added
    } else {
        AddRoot::Replaced { removed }
    })
}

/// Removes a root; nothing on disk is touched. Returns whether it was there.
pub fn remove_root(roots: &mut Roots, path: &Path) -> bool {
    let before = roots.paths.len();
    let key = path_key(path);
    roots.paths.retain(|root| path_key(root) != key);
    roots.paths.len() != before
}

/// Whether `path` falls under an exclusion: a folder path (and everything inside it)
/// or a glob pattern such as `**/vendor` (the matching folder and everything inside).
pub fn is_excluded(path: &Path, exclusions: &[String]) -> bool {
    let text = path.to_string_lossy();
    exclusions.iter().any(|rule| {
        if rule.contains('*') {
            let rule = rule.trim_end_matches(['/', '\\']);
            glob::matches(rule, &text) || glob::matches(&format!("{rule}/**"), &text)
        } else {
            contains(Path::new(rule), path)
        }
    })
}

/// Roots that can be analyzed now, plus a warning for each one that cannot
/// (e.g. an unplugged external drive). An unavailable root is not an error.
pub fn available(roots: &Roots) -> (Vec<PathBuf>, Vec<Message>) {
    let mut ready = Vec::new();
    let mut warnings = Vec::new();
    for root in &roots.paths {
        if fs::read_dir(root).is_ok() {
            ready.push(root.clone());
        } else {
            warnings.push(Message::new("roots.unavailable").with("path", root.display()));
        }
    }
    (ready, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folders(names: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in names {
            fs::create_dir_all(dir.path().join(name)).unwrap();
        }
        dir
    }

    #[test]
    fn t02_02_ca1_root_inside_another_root_is_not_duplicated() {
        let dir = folders(&["Projetos/estudo"]);
        let projetos = dir.path().join("Projetos");
        let mut roots = Roots {
            paths: vec![projetos.clone()],
            ..Roots::default()
        };

        let outcome = add_root(&mut roots, &projetos.join("estudo")).unwrap();
        assert_eq!(
            outcome,
            AddRoot::AlreadyCovered {
                by: projetos.clone()
            }
        );
        assert_eq!(roots.paths, vec![projetos]);
    }

    #[test]
    fn t02_02_ca2_parent_root_replaces_the_roots_inside_it() {
        let dir = folders(&["a/x", "a/y", "b"]);
        let (a, x, y, b) = (
            dir.path().join("a"),
            dir.path().join("a/x"),
            dir.path().join("a/y"),
            dir.path().join("b"),
        );
        let mut roots = Roots {
            paths: vec![x.clone(), b.clone(), y.clone()],
            ..Roots::default()
        };

        let outcome = add_root(&mut roots, &a).unwrap();
        assert_eq!(
            outcome,
            AddRoot::Replaced {
                removed: vec![x, y]
            }
        );
        assert_eq!(roots.paths, vec![b, a]);
    }

    #[test]
    fn unrelated_root_is_simply_added() {
        let dir = folders(&["a", "b"]);
        let mut roots = Roots::default();
        assert_eq!(
            add_root(&mut roots, &dir.path().join("a")),
            Ok(AddRoot::Added)
        );
        assert_eq!(
            add_root(&mut roots, &dir.path().join("b")),
            Ok(AddRoot::Added)
        );
        assert_eq!(roots.paths.len(), 2);
    }

    #[test]
    fn t02_02_ca3_excluded_paths_and_patterns_cover_everything_inside() {
        let exclusions = vec!["C:\\Projetos\\cliente-x".to_owned(), "**/vendor".to_owned()];
        assert!(is_excluded(
            Path::new("C:\\Projetos\\cliente-x"),
            &exclusions
        ));
        assert!(is_excluded(
            Path::new("C:\\Projetos\\cliente-x\\api\\node_modules"),
            &exclusions
        ));
        assert!(is_excluded(
            Path::new("C:\\Projetos\\loja\\vendor"),
            &exclusions
        ));
        assert!(is_excluded(
            Path::new("C:\\Projetos\\loja\\vendor\\pkg"),
            &exclusions
        ));
        assert!(!is_excluded(
            Path::new("C:\\Projetos\\cliente-xy"),
            &exclusions
        ));
        assert!(!is_excluded(
            Path::new("C:\\Projetos\\loja\\vendors"),
            &exclusions
        ));
    }

    #[test]
    fn t02_02_ca4_unavailable_root_only_produces_a_warning() {
        let dir = folders(&["ok"]);
        let missing = dir.path().join("disco-externo");
        let roots = Roots {
            paths: vec![dir.path().join("ok"), missing.clone()],
            ..Roots::default()
        };

        let (ready, warnings) = available(&roots);
        assert_eq!(ready, vec![dir.path().join("ok")]);
        assert_eq!(
            warnings,
            vec![Message::new("roots.unavailable").with("path", missing.display())]
        );
    }

    #[test]
    fn t02_02_ca5_nonexistent_path_is_refused() {
        let dir = folders(&[]);
        let mut roots = Roots::default();
        assert_eq!(
            add_root(&mut roots, &dir.path().join("nao-existe")),
            Err(RootError::NotFound)
        );
        assert!(roots.paths.is_empty());
    }

    #[test]
    fn files_are_not_valid_roots() {
        let dir = folders(&[]);
        let file = dir.path().join("arquivo.txt");
        fs::write(&file, "x").unwrap();
        assert_eq!(
            add_root(&mut Roots::default(), &file),
            Err(RootError::NotReadable)
        );
    }

    #[test]
    fn removing_a_root_touches_nothing_on_disk() {
        let dir = folders(&["a"]);
        let a = dir.path().join("a");
        let mut roots = Roots {
            paths: vec![a.clone()],
            ..Roots::default()
        };
        assert!(remove_root(&mut roots, &a));
        assert!(!remove_root(&mut roots, &a));
        assert!(roots.paths.is_empty());
        assert!(a.exists());
    }

    #[test]
    fn containment_compares_whole_segments() {
        assert!(contains(Path::new("C:/a"), Path::new("C:/a")));
        assert!(contains(Path::new("C:/a/"), Path::new("C:\\a\\b")));
        assert!(!contains(Path::new("C:/a"), Path::new("C:/ab")));
        assert!(!contains(Path::new("C:/a/b"), Path::new("C:/a")));
    }
}
