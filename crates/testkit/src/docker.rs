use limpador_core::ports::{CommandOutput, CommandRunner};
use std::io;
use std::sync::Mutex;

/// An image known to the fake Docker daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeImage {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size_bytes: u64,
}

impl FakeImage {
    pub fn new(id: &str, repository: &str, tag: &str, size_bytes: u64) -> Self {
        Self {
            id: id.to_owned(),
            repository: repository.to_owned(),
            tag: tag.to_owned(),
            size_bytes,
        }
    }
}

/// Docker double: answers like the `docker` CLI from configured state and records
/// every command it receives. It grows with the Docker collector (F03); commands it
/// does not know yet fail loudly instead of pretending to succeed.
#[derive(Debug, Default)]
pub struct FakeDocker {
    images: Mutex<Vec<FakeImage>>,
    calls: Mutex<Vec<Vec<String>>>,
}

impl FakeDocker {
    pub fn with_images(images: Vec<FakeImage>) -> Self {
        Self {
            images: Mutex::new(images),
            calls: Mutex::default(),
        }
    }

    /// Every command received, as `["docker", arg, ...]`.
    pub fn calls(&self) -> Vec<Vec<String>> {
        self.calls.lock().unwrap().clone()
    }

    pub fn images(&self) -> Vec<FakeImage> {
        self.images.lock().unwrap().clone()
    }

    fn list_images(&self) -> CommandOutput {
        // One JSON object per line, like `docker image ls --format json`.
        let stdout = self
            .images
            .lock()
            .unwrap()
            .iter()
            .map(|i| {
                format!(
                    r#"{{"ID":"{}","Repository":"{}","Tag":"{}","Size":"{}B"}}"#,
                    i.id, i.repository, i.tag, i.size_bytes
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        ok(stdout)
    }

    fn remove_image(&self, id: &str) -> CommandOutput {
        let mut images = self.images.lock().unwrap();
        match images.iter().position(|i| i.id == id) {
            Some(index) => {
                images.remove(index);
                ok(format!("Deleted: {id}"))
            }
            None => fail(format!("Error response from daemon: No such image: {id}")),
        }
    }
}

impl CommandRunner for FakeDocker {
    fn run(&self, program: &str, args: &[&str]) -> io::Result<CommandOutput> {
        let mut call = vec![program.to_owned()];
        call.extend(args.iter().map(|a| (*a).to_owned()));
        self.calls.lock().unwrap().push(call);

        if program != "docker" {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("FakeDocker only answers `docker`, got `{program}`"),
            ));
        }
        Ok(match args {
            ["image", "ls", "--format", "json"] => self.list_images(),
            ["image", "rm", id] => self.remove_image(id),
            _ => fail(format!("FakeDocker does not support: {}", args.join(" "))),
        })
    }
}

fn ok(stdout: String) -> CommandOutput {
    CommandOutput {
        status: 0,
        stdout,
        stderr: String::new(),
    }
}

fn fail(stderr: String) -> CommandOutput {
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_images() -> FakeDocker {
        FakeDocker::with_images(vec![
            FakeImage::new("sha256:aaa", "node", "18", 1_000),
            FakeImage::new("sha256:bbb", "postgres", "16", 2_000),
            FakeImage::new("sha256:ccc", "<none>", "<none>", 3_000),
        ])
    }

    #[test]
    fn t00_02_ca4_removing_an_image_is_recorded_and_listing_shrinks() {
        let docker = three_images();

        let removed = docker
            .run("docker", &["image", "rm", "sha256:bbb"])
            .unwrap();
        assert!(removed.success());

        assert_eq!(
            docker.calls(),
            vec![vec!["docker", "image", "rm", "sha256:bbb"]]
        );
        let listed = docker
            .run("docker", &["image", "ls", "--format", "json"])
            .unwrap();
        assert_eq!(listed.stdout.lines().count(), 2);
        assert!(!listed.stdout.contains("sha256:bbb"));
        assert!(listed.stdout.contains(r#""Repository":"node""#));
    }

    #[test]
    fn removing_unknown_image_fails_like_docker() {
        let docker = three_images();
        let out = docker
            .run("docker", &["image", "rm", "sha256:zzz"])
            .unwrap();
        assert!(!out.success());
        assert!(out.stderr.contains("No such image"));
        assert_eq!(docker.images().len(), 3);
    }

    #[test]
    fn unsupported_commands_fail_instead_of_pretending() {
        let docker = three_images();
        let out = docker.run("docker", &["system", "prune", "-f"]).unwrap();
        assert!(!out.success());
        assert_eq!(docker.images().len(), 3);
    }

    #[test]
    fn other_programs_are_rejected() {
        assert!(three_images().run("git", &["status"]).is_err());
    }
}
