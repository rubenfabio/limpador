//! Phantom space of the Docker Desktop and WSL virtual disks (T-03.06): a `.vhdx` grows
//! but never shrinks, so space freed inside stays taken on Windows until compaction.
//! Measuring never starts a distro: unmeasured disks are reported as such.

use crate::collector::{
    AnalysisContext, CleanOutcome, Collector, CollectorDescriptor, CollectorError, Detection,
    KindSpec, Revalidation,
};
use crate::item::{Category, CleanAction, Item, ItemDraft, Nature, Resource, Risk};
use crate::message::Message;
use crate::ports::CommandRunner;
use crate::risk::Destination;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const ID: &str = "wsl-disks";
/// Phantom space below this is not worth a compaction.
pub const MIN_PHANTOM: u64 = 1_000_000_000;
const LXSS: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss";
const DOCKER_DISTRO: &str = "docker-desktop";
/// Where Docker Desktop mounts its data disk inside its distro.
const DOCKER_DATA_MOUNT: &str = "/mnt/docker-desktop-disk";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distro {
    pub name: String,
    pub running: bool,
    pub version: u8,
}

/// `wsl -l -v` (with WSL_UTF8=1): "* Ubuntu   Running   2".
pub fn parse_distros(text: &str) -> Vec<Distro> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut parts = line.trim_start_matches(['*', ' ']).split_whitespace();
            let (name, state, version) = (parts.next()?, parts.next()?, parts.next()?);
            Some(Distro {
                name: name.to_owned(),
                running: state.eq_ignore_ascii_case("running"),
                version: version.parse().ok()?,
            })
        })
        .collect()
}

/// `reg query <Lxss> /s`: distro name → folder holding its virtual disk.
pub fn parse_registry(text: &str) -> Vec<(String, PathBuf, String)> {
    let mut found = Vec::new();
    let (mut name, mut base, mut vhd) = (None, None, None);
    let mut flush =
        |name: &mut Option<String>, base: &mut Option<String>, vhd: &mut Option<String>| {
            if let (Some(n), Some(b)) = (name.take(), base.take()) {
                let b = b.strip_prefix(r"\\?\").unwrap_or(&b).to_owned();
                found.push((
                    n,
                    PathBuf::from(b),
                    vhd.take().unwrap_or_else(|| "ext4.vhdx".into()),
                ));
            }
            *vhd = None;
        };
    for line in text.lines() {
        if line.starts_with("HKEY_") {
            flush(&mut name, &mut base, &mut vhd);
            continue;
        }
        let mut parts = line.trim().splitn(3, "    ");
        let (Some(key), Some(_kind), Some(value)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let value = value.trim().to_owned();
        match key {
            "DistributionName" => name = Some(value),
            "BasePath" => base = Some(value),
            "VhdFileName" => vhd = Some(value),
            _ => {}
        }
    }
    flush(&mut name, &mut base, &mut vhd);
    found
}

/// Used bytes from `df -B1 <path>` (second line, third column).
pub fn parse_df_used(text: &str) -> Option<u64> {
    text.lines().nth(1)?.split_whitespace().nth(2)?.parse().ok()
}

pub struct WslDisksCollector {
    commands: Arc<dyn CommandRunner>,
    /// Docker Desktop's data disk (from the catalog, ADR-0005).
    docker_data_disk: PathBuf,
}

/// A virtual disk and, when its distro is running, how much of it is really used.
struct Disk {
    label: String,
    distro: String,
    file: PathBuf,
    mount: &'static str,
}

impl WslDisksCollector {
    pub fn new(commands: Arc<dyn CommandRunner>, docker_data_disk: PathBuf) -> Self {
        Self {
            commands,
            docker_data_disk,
        }
    }

    fn run(&self, program: &str, args: &[&str]) -> Option<String> {
        self.commands
            .run(program, args)
            .ok()
            .filter(|o| o.success())
            .map(|o| o.stdout)
    }

    fn disks(&self, distros: &[Distro]) -> Vec<Disk> {
        let mut disks = Vec::new();
        if self.docker_data_disk.is_file() {
            disks.push(Disk {
                label: "docker-desktop (dados)".into(),
                distro: DOCKER_DISTRO.into(),
                file: self.docker_data_disk.clone(),
                mount: DOCKER_DATA_MOUNT,
            });
        }
        let registry = self.run("reg", &["query", LXSS, "/s"]).unwrap_or_default();
        for (name, base, vhd) in parse_registry(&registry) {
            let is_v2 = distros.iter().any(|d| d.name == name && d.version == 2);
            let file = base.join(vhd);
            if is_v2 && !name.starts_with(DOCKER_DISTRO) && file.is_file() {
                disks.push(Disk {
                    label: name.clone(),
                    distro: name,
                    file,
                    mount: "/",
                });
            }
        }
        disks
    }

    fn item(&self, disk: &Disk, distros: &[Distro]) -> Option<Item> {
        let size = fs::metadata(&disk.file).ok()?.len();
        let running = distros.iter().any(|d| d.name == disk.distro && d.running);
        let used = running
            .then(|| self.run("wsl", &["-d", &disk.distro, "-e", "df", "-B1", disk.mount]))
            .flatten()
            .and_then(|out| parse_df_used(&out));

        let effect = if disk.distro == DOCKER_DISTRO {
            Message::new("effect.stops_docker")
        } else {
            Message::new("effect.stops_wsl").with("distro", &disk.distro)
        };
        let draft = ItemDraft::new(
            ID,
            "virtual_disk",
            Category::Docker,
            Resource::Path(disk.file.clone()),
            &disk.label,
            Nature::Recreatable,
            Risk::Safe,
        )
        .action(CleanAction::Compact)
        .side_effect(effect)
        .recreate(Message::new("recreate.compaction_keeps_data"));

        let item = match used {
            Some(used) => {
                let phantom = size.saturating_sub(used);
                if phantom < MIN_PHANTOM {
                    return None;
                }
                draft.sizes(size, phantom).reason(
                    Message::new("reason.vdisk_phantom")
                        .with("size", size)
                        .with("used", used),
                )
            }
            // Not measured: shows the file size only; measuring would mean starting it.
            None => draft
                .sizes(size, 0)
                .size_estimated()
                .reason(Message::new("reason.vdisk_not_measured").with("size", size))
                .alert(Message::new("alert.start_to_measure").with("distro", &disk.distro)),
        };
        item.build().ok()
    }
}

impl Collector for WslDisksCollector {
    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor {
            id: ID,
            kinds: vec![KindSpec {
                kind: "virtual_disk",
                category: Category::Docker,
                nature: Nature::Recreatable,
                restorable: false,
            }],
            catalog_locations: vec!["docker.data_disk".into(), "wsl.registry".into()],
        }
    }

    fn detect(&self, _ctx: &AnalysisContext) -> Detection {
        match self.commands.run("wsl", &["-l", "-v"]) {
            Ok(out) if out.success() => Detection::Available,
            _ => Detection::NotInstalled,
        }
    }

    fn analyze(
        &self,
        ctx: &AnalysisContext,
        emit: &mut dyn FnMut(Item),
    ) -> Result<(), CollectorError> {
        let distros = parse_distros(&self.run("wsl", &["-l", "-v"]).unwrap_or_default());
        let disks = self.disks(&distros);
        for (done, disk) in disks.iter().enumerate() {
            if ctx.should_stop() {
                break;
            }
            if let Some(item) = self.item(disk, &distros) {
                emit(item);
            }
            ctx.report(done as u64 + 1, Some(disks.len() as u64));
        }
        Ok(())
    }

    fn revalidate(&self, _ctx: &AnalysisContext, item: &Item) -> Revalidation {
        let distros = parse_distros(&self.run("wsl", &["-l", "-v"]).unwrap_or_default());
        self.disks(&distros)
            .iter()
            .filter(|d| Resource::Path(d.file.clone()) == item.resource)
            .find_map(|d| self.item(d, &distros))
            .map_or(Revalidation::Gone, |i| {
                Revalidation::StillValid(Box::new(i))
            })
    }

    /// Compaction arrives with T-03.07 (it needs F09's confirmation flow).
    fn clean(
        &self,
        _item: &Item,
        _destination: Destination,
    ) -> Result<CleanOutcome, CollectorError> {
        Err(CollectorError::Unsupported)
    }
}

/// Where Docker Desktop keeps its data disk for a given `%LOCALAPPDATA%`.
pub fn docker_data_disk(local_app_data: &Path) -> PathBuf {
    local_app_data.join(r"Docker\wsl\disk\docker_data.vhdx")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::{StopSignal, tests::context};
    use crate::collectors::scripted::Scripted;
    use crate::risk::assess;

    const GB: u64 = 1_000_000_000;

    struct Machine {
        _dir: tempfile::TempDir,
        docker_disk: PathBuf,
        ubuntu_base: PathBuf,
    }

    /// Sparse files stand in for the `.vhdx` (sizes without using the disk).
    fn machine(docker_size: u64, ubuntu_size: u64) -> Machine {
        let dir = tempfile::tempdir().unwrap();
        let docker_disk = docker_data_disk(dir.path());
        let ubuntu_base = dir.path().join("wsl").join("{95bde519}");
        fs::create_dir_all(docker_disk.parent().unwrap()).unwrap();
        fs::create_dir_all(&ubuntu_base).unwrap();
        fs::File::create(&docker_disk)
            .unwrap()
            .set_len(docker_size)
            .unwrap();
        fs::File::create(ubuntu_base.join("ext4.vhdx"))
            .unwrap()
            .set_len(ubuntu_size)
            .unwrap();
        Machine {
            _dir: dir,
            docker_disk,
            ubuntu_base,
        }
    }

    fn registry(machine: &Machine) -> String {
        format!(
            "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\r\n    DefaultVersion    REG_DWORD    0x2\r\n\r\n\
             HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\\{{95bde519}}\r\n    State    REG_DWORD    0x1\r\n    DistributionName    REG_SZ    Ubuntu\r\n    Version    REG_DWORD    0x2\r\n    BasePath    REG_SZ    {}\r\n    VhdFileName    REG_SZ    ext4.vhdx\r\n\r\n\
             HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\\{{abacf557}}\r\n    DistributionName    REG_SZ    docker-desktop\r\n    BasePath    REG_SZ    \\\\?\\C:\\x\\Docker\\wsl\\main\r\n",
            machine.ubuntu_base.display()
        )
    }

    fn df(used: u64) -> String {
        format!(
            "Filesystem 1B-blocks Used Available Use% Mounted on\n/dev/sdd 1081101176832 {used} 1015053164544 1% /x\n"
        )
    }

    fn list(ubuntu: &str, docker: &str) -> String {
        format!(
            "  NAME              STATE           VERSION\n* Ubuntu            {ubuntu}         2\n  docker-desktop    {docker}         2\n"
        )
    }

    fn scripted(machine: &Machine, ubuntu: &str, docker: &str) -> Scripted {
        Scripted::default()
            .ok("wsl -l -v", &list(ubuntu, docker))
            .ok(&format!("reg query {LXSS} /s"), &registry(machine))
            .ok(
                "wsl -d docker-desktop -e df -B1 /mnt/docker-desktop-disk",
                &df(5 * GB),
            )
            .ok("wsl -d Ubuntu -e df -B1 /", &df(10 * GB))
    }

    fn analyze(commands: Arc<Scripted>, machine: &Machine) -> Vec<Item> {
        let collector = WslDisksCollector::new(commands, machine.docker_disk.clone());
        let mut items = vec![];
        collector
            .analyze(&context(StopSignal::new(None)), &mut |i| items.push(i))
            .unwrap();
        items
    }

    #[test]
    fn t03_06_ca1_phantom_space_is_file_size_minus_used_space() {
        let machine = machine(17 * GB, 11 * GB);
        let items = analyze(Arc::new(scripted(&machine, "Running", "Running")), &machine);
        let docker = items
            .iter()
            .find(|i| i.name == "docker-desktop (dados)")
            .unwrap();
        assert_eq!(
            (docker.apparent_size, docker.freeable_size),
            (17 * GB, 12 * GB)
        );
        assert_eq!(docker.action, CleanAction::Compact);
        assert_eq!(
            docker.side_effects,
            vec![Message::new("effect.stops_docker")]
        );
        assert_eq!(
            docker.reason,
            Message::new("reason.vdisk_phantom")
                .with("size", 17 * GB)
                .with("used", 5 * GB)
        );
        assert!(
            !assess(docker, None).automation_eligible,
            "compaction is never automatic"
        );
    }

    #[test]
    fn t03_06_ca2_stopped_docker_disk_is_not_measured_and_nothing_is_started() {
        let machine = machine(17 * GB, 11 * GB);
        let commands = Arc::new(scripted(&machine, "Stopped", "Stopped"));
        let items = analyze(Arc::clone(&commands), &machine);

        let docker = items
            .iter()
            .find(|i| i.name == "docker-desktop (dados)")
            .unwrap();
        assert!(docker.size_estimated);
        assert_eq!((docker.apparent_size, docker.freeable_size), (17 * GB, 0));
        assert_eq!(docker.reason.key, "reason.vdisk_not_measured");
        assert!(
            commands.calls().iter().all(|c| !c.contains("-e df")),
            "no distro was started"
        );
    }

    #[test]
    fn t03_06_ca3_less_than_one_gb_of_phantom_space_is_not_shown() {
        let machine = machine(5 * GB + 300_000_000, 11 * GB);
        let items = analyze(Arc::new(scripted(&machine, "Stopped", "Running")), &machine);
        assert!(items.iter().all(|i| i.name != "docker-desktop (dados)"));
    }

    #[test]
    fn t03_06_ca5_each_wsl_distro_is_its_own_item() {
        let machine = machine(17 * GB, 11 * GB);
        let items = analyze(Arc::new(scripted(&machine, "Running", "Stopped")), &machine);
        let ubuntu = items.iter().find(|i| i.name == "Ubuntu").unwrap();
        assert_eq!(ubuntu.freeable_size, GB);
        assert_eq!(
            ubuntu.side_effects,
            vec![Message::new("effect.stops_wsl").with("distro", "Ubuntu")]
        );
        assert_eq!(
            items.len(),
            2,
            "docker-desktop's own system disk is not listed"
        );
    }

    #[test]
    fn t03_01_ca1_without_docker_installed_wsl_disks_are_still_analyzed() {
        let machine = machine(17 * GB, 11 * GB);
        fs::remove_file(&machine.docker_disk).unwrap();
        let items = analyze(Arc::new(scripted(&machine, "Running", "Stopped")), &machine);
        assert_eq!(
            items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            vec!["Ubuntu"]
        );
    }

    #[test]
    fn without_wsl_the_collector_is_not_installed() {
        let collector = WslDisksCollector::new(Arc::new(Scripted::default()), "C:\\x".into());
        assert_eq!(
            collector.detect(&context(StopSignal::new(None))),
            Detection::NotInstalled
        );
        let ok = WslDisksCollector::new(
            Arc::new(Scripted::default().ok("wsl -l -v", "  NAME STATE VERSION\n")),
            "C:\\x".into(),
        );
        assert_eq!(
            ok.detect(&context(StopSignal::new(None))),
            Detection::Available
        );
    }

    #[test]
    fn revalidation_measures_again_and_compaction_is_not_available_yet() {
        let machine = machine(17 * GB, 11 * GB);
        let commands = Arc::new(scripted(&machine, "Running", "Running"));
        let items = analyze(Arc::clone(&commands), &machine);
        let collector = WslDisksCollector::new(commands, machine.docker_disk.clone());
        let ctx = context(StopSignal::new(None));
        let docker = items
            .iter()
            .find(|i| i.name == "docker-desktop (dados)")
            .unwrap();

        assert!(matches!(
            collector.revalidate(&ctx, docker),
            Revalidation::StillValid(_)
        ));
        fs::remove_file(&machine.docker_disk).unwrap();
        assert_eq!(collector.revalidate(&ctx, docker), Revalidation::Gone);
        assert_eq!(
            collector.clean(docker, Destination::Direct),
            Err(CollectorError::Unsupported)
        );
    }

    #[test]
    fn parsers_read_the_real_formats() {
        assert_eq!(
            parse_distros(
                "  NAME   STATE   VERSION\r\n* Ubuntu   Running   2\r\n  legado  Stopped  1\r\n"
            ),
            vec![
                Distro {
                    name: "Ubuntu".into(),
                    running: true,
                    version: 2
                },
                Distro {
                    name: "legado".into(),
                    running: false,
                    version: 1
                },
            ]
        );
        assert_eq!(parse_df_used(&df(42)), Some(42));
        assert_eq!(parse_df_used("cabeçalho só"), None);
        let reg = parse_registry(
            "HKEY_X\\{a}\r\n    DistributionName    REG_SZ    Debian\r\n    BasePath    REG_SZ    \\\\?\\D:\\wsl\\debian\r\n",
        );
        assert_eq!(
            reg,
            vec![(
                "Debian".into(),
                PathBuf::from("D:\\wsl\\debian"),
                "ext4.vhdx".into()
            )]
        );
    }

    #[test]
    fn only_wsl2_distros_with_an_existing_disk_are_listed() {
        let machine = machine(17 * GB, 11 * GB);
        let root = machine.ubuntu_base.parent().unwrap().to_path_buf();
        // Every candidate has a real disk file, except "sem-disco".
        for (folder, file) in [
            ("legado", "ext4.vhdx"),
            ("fantasma", "ext4.vhdx"),
            ("custom", "disco.vhdx"),
            ("docker", "ext4.vhdx"),
        ] {
            fs::create_dir_all(root.join(folder)).unwrap();
            fs::File::create(root.join(folder).join(file))
                .unwrap()
                .set_len(3 * GB)
                .unwrap();
        }
        let key = |name: &str, folder: &str, vhd: &str| {
            format!(
                "HKEY_X\\{{{name}}}\r\n    DistributionName    REG_SZ    {name}\r\n    BasePath    REG_SZ    {}\r\n{vhd}",
                root.join(folder).display()
            )
        };
        let registry = [
            key("legado", "legado", ""),
            key("fantasma", "fantasma", ""),
            key(
                "custom",
                "custom",
                "    VhdFileName    REG_SZ    disco.vhdx\r\n",
            ),
            key("docker-desktop-data", "docker", ""),
            key("sem-disco", "nada", ""),
        ]
        .concat();
        let listed = "  NAME   STATE   VERSION\n  legado   Stopped   1\n  custom   Stopped   2\n  docker-desktop-data   Stopped   2\n  sem-disco   Stopped   2\n";
        let commands = Arc::new(
            Scripted::default()
                .ok("wsl -l -v", listed)
                .ok(&format!("reg query {LXSS} /s"), &registry),
        );
        fs::remove_file(&machine.docker_disk).unwrap();
        let progress = Arc::new(std::sync::Mutex::new(vec![]));
        let sink = Arc::clone(&progress);
        let ctx = AnalysisContext::new(
            vec![],
            Arc::new(crate::ports::SystemClock),
            commands.clone(),
            StopSignal::new(None),
            Arc::new(move |p| sink.lock().unwrap().push((p.done, p.total))),
        );
        let mut items = vec![];
        WslDisksCollector::new(commands, machine.docker_disk.clone())
            .analyze(&ctx, &mut |i| items.push(i))
            .unwrap();

        assert_eq!(
            items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            vec!["custom"],
            "WSL 1, unregistered, docker-desktop and missing disks are skipped"
        );
        assert_eq!(
            items[0].resource,
            Resource::Path(root.join("custom").join("disco.vhdx"))
        );
        assert_eq!(*progress.lock().unwrap(), vec![(1, Some(1))]);
    }

    #[test]
    fn a_failing_wsl_means_not_installed() {
        let broken = WslDisksCollector::new(
            Arc::new(Scripted::default().answer("wsl -l -v", 1, "", "WSL não habilitado")),
            "C:\\x".into(),
        );
        assert_eq!(
            broken.detect(&context(StopSignal::new(None))),
            Detection::NotInstalled
        );
    }
}
