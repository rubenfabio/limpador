//! Build outputs of each project (T-04.02): `.next`, `dist`, `coverage`… grouped in one
//! item per project. Only folders git ignores are candidates; without git, only names that
//! cannot be anything else.

use super::in_use_signal;
use super::javascript::{NPM_WITHOUT_LOCK, has_script, locked_manager};
use crate::collector::{
    AnalysisContext, CleanOutcome, Collector, CollectorDescriptor, CollectorError, Detection,
    KindSpec, Revalidation,
};
use crate::item::{Category, Item, ItemDraft, Nature, Resource, Risk};
use crate::message::Message;
use crate::ports::{CommandRunner, Platform};
use crate::projects::{Project, ProjectGit, ends_with_folder, find_folders, project_at};
use crate::risk::Destination;
use crate::tree::{measure, remove};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const ID: &str = "build-outputs";
const KIND: &str = "build_output";
const MAX_DEPTH: u32 = 8;

/// Names that only build tools create: safe to offer even without git (T-04.02).
const UNAMBIGUOUS: &[&str] = &[
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".astro",
    ".turbo",
    ".parcel-cache",
    ".vite",
    ".angular/cache",
];

pub struct BuildOutputsCollector {
    commands: Arc<dyn CommandRunner>,
    platform: Arc<dyn Platform>,
    /// Folder names (or `a/b` paths) recognized as build outputs, from the preferences.
    folders: Vec<String>,
}

impl BuildOutputsCollector {
    pub fn new(
        commands: Arc<dyn CommandRunner>,
        platform: Arc<dyn Platform>,
        folders: Vec<String>,
    ) -> Self {
        Self {
            commands,
            platform,
            folders,
        }
    }

    fn recognized(&self, relative: &str) -> bool {
        self.folders.iter().any(|f| ends_with_folder(relative, f))
    }

    /// Recognized folders of `project` that are safe to offer.
    fn candidates(&self, project: &Project, others: &[&Path]) -> Vec<PathBuf> {
        let found = find_folders(&project.root, others, MAX_DEPTH, &|relative| {
            self.recognized(relative)
        });
        if found.is_empty() {
            return found;
        }
        if project.git == ProjectGit::None {
            return found
                .into_iter()
                .filter(|folder| {
                    let relative = relative_to(folder, &project.root);
                    UNAMBIGUOUS.iter().any(|u| ends_with_folder(&relative, u))
                })
                .collect();
        }
        let relative: Vec<String> = found
            .iter()
            .map(|f| relative_to(f, &project.root))
            .collect();
        // Names with accents come unquoted: with quotePath off (check-ignore) and with -z
        // (ls-files). Windows forbids the few characters git would still quote.
        let ignored = self.git_paths(
            &project.root,
            &["-c", "core.quotePath=false", "check-ignore"],
            &relative,
            '\n',
        );
        let tracked = self.git_paths(&project.root, &["ls-files", "-z"], &relative, '\0');
        found
            .into_iter()
            .zip(relative)
            .filter(|(_, rel)| ignored.contains(rel))
            // `check-ignore` answers for the folder's name even when files inside it are
            // versioned: a folder with any tracked file is never a candidate.
            .filter(|(_, rel)| {
                let inside = format!("{rel}/");
                tracked.iter().all(|file| !file.starts_with(&inside))
            })
            .map(|(folder, _)| folder)
            .collect()
    }

    /// Paths printed by `git -C root <command> -- <paths>`, split at `separator`. Any
    /// failure gives none, which keeps every folder out (the safe answer).
    fn git_paths(
        &self,
        root: &Path,
        command: &[&str],
        paths: &[String],
        separator: char,
    ) -> HashSet<String> {
        let root_text = root.display().to_string();
        let mut args = vec!["-C", root_text.as_str()];
        args.extend(command);
        args.push("--");
        args.extend(paths.iter().map(String::as_str));
        match self.commands.run("git", &args) {
            // check-ignore exits 1 when nothing is ignored: no paths either way.
            Ok(out) if out.status == 0 => out
                .stdout
                .split(separator)
                .map(|path| path.trim_end_matches(['\r', '/']))
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect(),
            _ => HashSet::new(),
        }
    }

    fn item(&self, project: &Project, others: &[&Path], ctx: &AnalysisContext) -> Option<Item> {
        let folders = self.candidates(project, others);
        if folders.is_empty() {
            return None;
        }
        let size = measure(&folders, self.platform.as_ref(), &ctx.stop);
        let recreate = if has_script(&project.root, "build") {
            let manager = locked_manager(&project.root).unwrap_or(NPM_WITHOUT_LOCK);
            Message::new("recreate.run_command")
                .with("command", format!("{} run build", manager.program))
        } else {
            Message::new("recreate.next_build")
        };
        let mut draft = ItemDraft::new(
            ID,
            KIND,
            Category::DependenciesAndBuilds,
            Resource::Path(project.root.clone()),
            format!("{} · build", project.name),
            Nature::Recreatable,
            Risk::Moderate,
        )
        .sizes(size.apparent, size.freeable)
        .reason(Message::new("reason.build_outputs").with("count", folders.len()))
        .recreate(recreate)
        .project_hint(&project.root);
        for folder in folders {
            draft = draft.part(folder);
        }
        draft.build().ok()
    }

    fn projects(ctx: &AnalysisContext) -> Vec<&Project> {
        ctx.discovery
            .projects
            .iter()
            .chain(&ctx.discovery.worktrees)
            .collect()
    }

    /// Projects nested inside `root`, which have outputs of their own.
    fn nested<'a>(ctx: &'a AnalysisContext, root: &Path) -> Vec<&'a Path> {
        Self::projects(ctx)
            .into_iter()
            .map(|p| p.root.as_path())
            .filter(|r| *r != root && r.starts_with(root))
            .collect()
    }
}

/// `folder` relative to `root`, with `/` like git and the folder list use.
fn relative_to(folder: &Path, root: &Path) -> String {
    folder
        .strip_prefix(root)
        .unwrap_or(folder)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

impl Collector for BuildOutputsCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor {
            id: ID,
            kinds: vec![KindSpec {
                kind: KIND,
                category: Category::DependenciesAndBuilds,
                nature: Nature::Recreatable,
                restorable: false,
            }],
            catalog_locations: vec![],
        }
    }

    fn detect(&self, _ctx: &AnalysisContext) -> Detection {
        Detection::Available
    }

    fn analyze(
        &self,
        ctx: &AnalysisContext,
        emit: &mut dyn FnMut(Item),
    ) -> Result<(), CollectorError> {
        let projects = Self::projects(ctx);
        for (done, project) in projects.iter().enumerate() {
            if ctx.should_stop() {
                break;
            }
            let others = Self::nested(ctx, &project.root);
            if let Some(item) = self.item(project, &others, ctx) {
                emit(item);
            }
            ctx.report(done as u64 + 1, Some(projects.len() as u64));
        }
        Ok(())
    }

    fn revalidate(&self, ctx: &AnalysisContext, item: &Item) -> Revalidation {
        let Some(root) = &item.project_hint else {
            return Revalidation::Gone;
        };
        let project = Self::projects(ctx)
            .into_iter()
            .find(|p| &p.root == root)
            .cloned()
            .unwrap_or_else(|| project_at(root));
        let others = Self::nested(ctx, root);
        let Some(mut fresh) = self.item(&project, &others, ctx) else {
            return Revalidation::Gone;
        };
        if let Some(signal) = in_use_signal(&fresh.parts, root, self.platform.as_ref(), &ctx.stop) {
            fresh.signals.push(signal);
        }
        Revalidation::StillValid(Box::new(fresh))
    }

    fn clean(&self, item: &Item, destination: Destination) -> Result<CleanOutcome, CollectorError> {
        if destination != Destination::Direct {
            return Err(CollectorError::Unsupported);
        }
        // Only recognized build folders inside the item's own project are removed (I4).
        let root = item.project_hint.as_deref().unwrap_or(Path::new(""));
        let outside = item.parts.iter().any(|folder| {
            !folder.starts_with(root)
                || folder == root
                || !self.recognized(&relative_to(folder, root))
        });
        if item.parts.is_empty() || root.as_os_str().is_empty() || outside {
            return Err(CollectorError::Failed(Message::new("clean.outside_item")));
        }
        for folder in &item.parts {
            remove(folder).map_err(|e| {
                CollectorError::Failed(
                    Message::new("clean.remove_failed")
                        .with("path", folder.display())
                        .with("detail", e),
                )
            })?;
        }
        Ok(CleanOutcome::Cleaned {
            freed_bytes: item.freeable_size,
        })
    }
}

#[cfg(test)]
#[path = "build_outputs_tests.rs"]
mod tests;
