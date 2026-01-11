use clap::{crate_version, Arg, Command, ArgAction};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use uucore::error::{UResult, UError, UIoError};

const ABOUT: &str = "List information about block devices.";
const USAGE: &str = "lsblk [OPTIONS] [DEVICE]...";

#[derive(Debug, Clone)]
struct BlockDevice {
    name: String,
    major: u32,
    minor: u32,
    removable: bool,
    size: u64,
    ro: bool,
    device_type: String,
    mountpoint: Option<String>,
    children: Vec<BlockDevice>,
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args)?;

    let all = matches.get_flag("all");

    let mounts = read_mounts().unwrap_or_default();
    let devices = scan_devices(&mounts, all)?;

    print_all(&devices);

    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(USAGE)
        .infer_long_args(true)
        .arg(
            Arg::new("all")
                .short('a')
                .long("all")
                .help("Output all devices")
                .action(ArgAction::SetTrue),
        )
}

fn read_mounts() -> io::Result<Vec<(String, String)>> {
    let file = fs::File::open("/proc/mounts")?;
    let reader = io::BufReader::new(file);
    let mut mounts = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            mounts.push((parts[0].to_string(), parts[1].to_string()));
        }
    }
    Ok(mounts)
}

fn scan_devices(mounts: &[(String, String)], _all: bool) -> UResult<Vec<BlockDevice>> {
    let sys_class_block = Path::new("/sys/class/block");
    if !sys_class_block.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(sys_class_block).map_err(|e| Box::new(UIoError::from(e)) as Box<dyn UError>)?;

    let mut all_devs: Vec<String> = entries
        .filter_map(|e| e.ok().map(|d| d.file_name().to_string_lossy().to_string()))
        .collect();
    all_devs.sort();

    let mut roots = Vec::new();
    let mut parts_map: HashMap<String, Vec<String>> = HashMap::new();

    for name in &all_devs {
        let path = sys_class_block.join(name);
        
        let is_part = path.join("partition").exists();

        if is_part {
            if let Ok(canon) = fs::canonicalize(&path) {
                if let Some(parent) = canon.parent() {
                    if let Some(parent_name) = parent.file_name() {
                        let p_str = parent_name.to_string_lossy().to_string();
                        parts_map.entry(p_str).or_default().push(name.clone());
                    }
                }
            }
        } else {
            roots.push(name.clone());
        }
    }

    let mut devices = Vec::new();
    for name in roots {
        if let Some(dev) = build_device(&name, &parts_map, mounts) {
            devices.push(dev);
        }
    }

    Ok(devices)
}

fn build_device(
    name: &str,
    parts_map: &HashMap<String, Vec<String>>,
    mounts: &[(String, String)],
) -> Option<BlockDevice> {
    let path = Path::new("/sys/class/block").join(name);

    let size_str = fs::read_to_string(path.join("size")).ok()?;
    let size_sects = size_str.trim().parse::<u64>().ok()?;
    let size = size_sects * 512;

    let maj_min = fs::read_to_string(path.join("dev")).unwrap_or_default();
    let maj_min = maj_min.trim().to_string();
    let parts: Vec<&str> = maj_min.split(':').collect();
    let major = parts.get(0).unwrap_or(&"0").parse().unwrap_or(0);
    let minor = parts.get(1).unwrap_or(&"0").parse().unwrap_or(0);

    let removable = fs::read_to_string(path.join("removable"))
        .unwrap_or("0".to_string())
        .trim()
        == "1";
    let ro = fs::read_to_string(path.join("ro"))
        .unwrap_or("0".to_string())
        .trim()
        == "1";

    let device_type = if path.join("partition").exists() {
        "part".to_string()
    } else if name.starts_with("loop") {
        "loop".to_string()
    } else if name.starts_with("sr") {
        "rom".to_string()
    } else {
        "disk".to_string()
    };

    let dev_path = format!("/dev/{}", name);
    let mountpoint = mounts
        .iter()
        .find(|(src, _)| src == &dev_path)
        .map(|(_, dst)| dst.clone());

    let mut children = Vec::new();
    if let Some(kids) = parts_map.get(name) {
        for kid in kids {
            if let Some(child) = build_device(kid, parts_map, mounts) {
                children.push(child);
            }
        }
    }
    children.sort_by(|a, b| a.name.cmp(&b.name));

    Some(BlockDevice {
        name: name.to_string(),
        major,
        minor,
        removable,
        size,
        ro,
        device_type,
        mountpoint,
        children,
    })
}

fn print_all(devices: &[BlockDevice]) {
    println!(
        "{:<12} {:<8} {:<3} {:<8} {:<3} {:<5} {}",
        "NAME", "MAJ:MIN", "RM", "SIZE", "RO", "TYPE", "MOUNTPOINTS"
    );

    for dev in devices {
        print_dev_row(dev, "", false);
        print_children(&dev.children, "");
    }
}

fn print_children(devices: &[BlockDevice], indent: &str) {
    for (i, dev) in devices.iter().enumerate() {
        let is_last = i == devices.len() - 1;
        let connector = if is_last { "└─" } else { "├─" };

        let prefix = format!("{}{}", indent, connector);
        print_dev_row(dev, &prefix, true);

        let child_indent = format!("{}{}", indent, if is_last { "  " } else { "│ " });
        print_children(&dev.children, &child_indent);
    }
}

fn print_dev_row(dev: &BlockDevice, prefix: &str, _is_child: bool) {
    let name_display = format!("{}{}", prefix, dev.name);
    println!(
        "{:<12} {:>3}:{:<3}   {:<3} {:<8} {:<3} {:<5} {}",
        name_display,
        dev.major,
        dev.minor,
        if dev.removable { "1" } else { "0" },
        format_size(dev.size),
        if dev.ro { "1" } else { "0" },
        dev.device_type,
        dev.mountpoint.as_deref().unwrap_or("")
    );
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
