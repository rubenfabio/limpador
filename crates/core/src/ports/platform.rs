use std::io;
use std::path::Path;

/// Identity of a file on disk: every hardlink of a file has the same one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub volume: u64,
    pub index: u64,
    /// How many names (hardlinks) the file has.
    pub links: u32,
}

/// What the operating system knows about files beyond the standard library (ADR-0022).
pub trait Platform: Send + Sync {
    /// Identity of a regular file, without following links.
    fn file_identity(&self, path: &Path) -> io::Result<FileIdentity>;

    /// Whether another process holds `path` open in a way that prevents deleting it now.
    fn delete_blocked(&self, path: &Path) -> bool;

    /// Name of a process that has `path` open, when the system can tell.
    fn process_using(&self, path: &Path) -> Option<String>;
}
