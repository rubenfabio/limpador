//! Collectors: one per ecosystem (ADR-0004).

pub mod docker;
pub mod docker_cli;
pub mod node_modules;
#[cfg(test)]
pub(crate) mod scripted;
pub mod wsl_disks;
