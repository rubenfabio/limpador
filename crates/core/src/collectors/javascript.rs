//! What JavaScript projects tell about themselves: package manager and scripts.

use std::fs;
use std::path::Path;

/// Package manager of a project, from its lock file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageManager {
    pub program: &'static str,
    /// Reinstalls exactly what the lock file records.
    pub install: &'static str,
}

const LOCKS: [(&str, PackageManager); 5] = [
    (
        "pnpm-lock.yaml",
        PackageManager {
            program: "pnpm",
            install: "pnpm install",
        },
    ),
    (
        "yarn.lock",
        PackageManager {
            program: "yarn",
            install: "yarn install",
        },
    ),
    (
        "package-lock.json",
        PackageManager {
            program: "npm",
            install: "npm ci",
        },
    ),
    (
        "bun.lock",
        PackageManager {
            program: "bun",
            install: "bun install",
        },
    ),
    (
        "bun.lockb",
        PackageManager {
            program: "bun",
            install: "bun install",
        },
    ),
];

/// Without a lock file, npm is the one every Node installation has.
pub const NPM_WITHOUT_LOCK: PackageManager = PackageManager {
    program: "npm",
    install: "npm install",
};

/// The package manager whose lock file is in `root`, if any.
pub fn locked_manager(root: &Path) -> Option<PackageManager> {
    LOCKS
        .iter()
        .find(|(file, _)| root.join(file).is_file())
        .map(|(_, manager)| *manager)
}

/// Whether `package.json` in `root` declares the script `name`.
pub fn has_script(root: &Path, name: &str) -> bool {
    fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some_and(|json| json["scripts"][name].is_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lock_file_names_the_manager() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(locked_manager(dir.path()), None);
        fs::write(dir.path().join("yarn.lock"), "").unwrap();
        assert_eq!(locked_manager(dir.path()).unwrap().program, "yarn");
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(
            locked_manager(dir.path()),
            Some(PackageManager {
                program: "pnpm",
                install: "pnpm install"
            }),
            "pnpm wins when several locks exist"
        );
    }

    #[test]
    fn scripts_are_read_from_package_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!has_script(dir.path(), "build"));
        fs::write(dir.path().join("package.json"), "{ quebrado").unwrap();
        assert!(!has_script(dir.path(), "build"));
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"build": "vite build", "dev": 3}}"#,
        )
        .unwrap();
        assert!(has_script(dir.path(), "build"));
        assert!(!has_script(dir.path(), "dev"), "not a command");
        assert!(!has_script(dir.path(), "test"));
    }
}
