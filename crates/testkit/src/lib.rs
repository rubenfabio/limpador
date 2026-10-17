//! Fake machine for tests (ADR-0012).
//!
//! Builds a complete, disposable machine inside a temporary directory: projects with
//! controlled dates, real git repositories, junctions and Windows edge cases, plus
//! doubles for Docker and the clock. Everything is deleted when the [`FakeMachine`]
//! is dropped, even if the test panics.

mod clock;
mod docker;
mod git;
mod machine;
mod windows;

pub use clock::FixedClock;
pub use docker::{FakeDocker, FakeImage};
pub use git::{GitRepo, SquashMergedBranch};
pub use machine::{FakeMachine, MachineSnapshot, ProjectKind, ProjectSpec};
pub use windows::{LockedFile, create_junction};
