//! Collectors: one per ecosystem (ADR-0004).

pub mod build_outputs;
pub mod docker;
pub mod docker_cli;
pub mod javascript;
pub mod node_modules;
#[cfg(test)]
pub(crate) mod scripted;
pub mod wsl_disks;

use crate::collector::StopSignal;
use crate::message::Message;
use crate::ports::Platform;
use crate::risk::Signal;
use crate::tree::first_in_use;
use std::path::{Path, PathBuf};

/// ⛔ signal when a file under `folders` is held open by another process (I5), citing the
/// file (relative to `root`) and, when the system tells, the process.
pub(crate) fn in_use_signal(
    folders: &[PathBuf],
    root: &Path,
    platform: &dyn Platform,
    stop: &StopSignal,
) -> Option<Signal> {
    let file = first_in_use(folders, platform, stop)?;
    let shown = file
        .strip_prefix(root)
        .unwrap_or(&file)
        .display()
        .to_string();
    Some(Signal::InUse(match platform.process_using(&file) {
        Some(process) => Message::new("signal.file_in_use")
            .with("file", shown)
            .with("process", process),
        None => Message::new("signal.file_in_use_unknown").with("file", shown),
    }))
}
