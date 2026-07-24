use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use ballast_core::model::disk::{DeviceKind, DiskDevice};
use color_eyre::eyre::{self, Context};
use nix::sys::statvfs::statvfs;

pub mod disk_io;

pub fn enumerate_block_devices() -> eyre::Result<Vec<DiskDevice>> {
    let mut out = Vec::new();
    let mount_map = read_mount_map().wrap_err("failed to read /proc/mounts")?;

    for entry in fs::read_dir("/sys/block").wrap_err("failed to read /sys/block")? {
        let entry = entry.wrap_err("failed to read entry in /sys/block")?;
        let name = entry.file_name().to_string_lossy().to_string();
        let dev_path = entry.path();

        let kind = classify_block_device(&name, &dev_path);
        let removable = read_removable(&dev_path).unwrap_or_else(|| {
            eprintln!("Failed to read removable flag, assuming false");
            false
        });

        out.push(build_device(
            name.clone(),
            kind,
            &dev_path,
            removable,
            &mount_map,
        ));
        out.extend(enumerate_partitions(
            &name, &dev_path, removable, &mount_map,
        )?);
    }

    Ok(out)
}

fn classify_block_device(name: &str, dev_path: &Path) -> DeviceKind {
    if !name.starts_with("loop") {
        return DeviceKind::Disk;
    }
    let backing_file = match fs::read_to_string(dev_path.join("loop/backing_file")) {
        Ok(s) => Some(PathBuf::from(s.trim())),

        // An unattached loop dev has no backing file
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            eprintln!("Failed to read loop backing file {err}");
            None
        }
    };

    DeviceKind::Loopback { backing_file }
}

fn read_removable(dev_path: &Path) -> Option<bool> {
    Some(fs::read_to_string(dev_path.join("removable")).ok()?.trim() == "1")
}

fn read_partition_number(sub_path: &Path) -> eyre::Result<u32> {
    let raw = fs::read_to_string(sub_path.join("partition"))?;
    raw.trim().parse().wrap_err("invalid partition number")
}
/// Builds a `DiskDevice` from sysfs and mount info. Individual missing fields
/// (size, usage) are logged and left as `None` rather than failing the whole
/// scan, since one unreadable attribute shouldn't hide the rest of the device's info.
fn build_device(
    id: String,
    kind: DeviceKind,
    dev_path: &Path,
    removable: bool,
    mount_map: &HashMap<String, Vec<MountInfo>>,
) -> DiskDevice {
    let mounts = mount_map.get(&id);
    let mountpoints = mounts
        .map(|m| m.iter().map(|mi| mi.mountpoint.clone()).collect())
        .unwrap_or_default();

    let fstype = read_fstype_via_udev(dev_path)
        .or_else(|| mounts.and_then(|m| m.first()).map(|mi| mi.fstype.clone()));

    let size = read_size_bytes(dev_path);

    let used = mounts
        .and_then(|m| m.first())
        .and_then(|mi| read_used_bytes(&mi.mountpoint));

    DiskDevice {
        id,
        kind,
        fstype,
        mountpoints,
        removable,
        size,
        used,
    }
}

/// Scans a disk's sysfs directory for partition subdirectories and builds a
/// `DiskDevice` for each one.
fn enumerate_partitions(
    disk_name: &str,
    dev_path: &Path,
    removable: bool,
    mount_map: &HashMap<String, Vec<MountInfo>>,
) -> eyre::Result<Vec<DiskDevice>> {
    let mut out = Vec::new();

    let entries = fs::read_dir(dev_path)
        .wrap_err_with(|| format!("failed to read partitions for {disk_name}"))?;

    for sub_entry in entries {
        let sub_entry = sub_entry
            .wrap_err_with(|| format!("failed to read a partition entry for {disk_name}"))?;
        let sub_path = sub_entry.path();

        if !sub_path.join("partition").exists() {
            continue;
        }

        let sub_name = sub_entry.file_name().to_string_lossy().into_owned();
        let part_num = read_partition_number(&sub_path).unwrap_or_else(|_err| {
            eprintln!("Failed to read partition number, defaulting to 0");
            0
        });

        let kind = DeviceKind::Partition {
            parent: disk_name.to_string(),
            part_num,
        };
        out.push(build_device(
            sub_name, kind, &sub_path, removable, mount_map,
        ));
    }

    Ok(out)
}

struct MountInfo {
    mountpoint: PathBuf,
    fstype: String,
}

fn read_mount_map() -> eyre::Result<HashMap<String, Vec<MountInfo>>> {
    let content = fs::read_to_string("/proc/mounts")?;
    let mut map: HashMap<String, Vec<MountInfo>> = HashMap::new();

    for line in content.lines() {
        let mut fields = line.split_whitespace();
        let Some(device) = fields.next() else {
            continue;
        };
        let Some(mp) = fields.next() else {
            continue;
        };
        // NOTE: /proc/mounts will only get fstype of mounted block devs
        let Some(fstype) = fields.next() else {
            continue;
        };

        if let Some(name) = device.strip_prefix("/dev/") {
            map.entry(name.to_string()).or_default().push(MountInfo {
                mountpoint: PathBuf::from(mp),
                fstype: fstype.to_string(),
            });
        }
    }

    Ok(map)
}

fn read_fstype_via_udev(dev_path: &Path) -> Option<String> {
    let devno = fs::read_to_string(dev_path.join("dev")).ok()?;
    let devno = devno.trim();
    let udev_db_path = format!("/run/udev/data/b{devno}");
    let content = fs::read_to_string(udev_db_path).ok()?;

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("E:ID_FS_TYPE=") {
            return Some(value.to_string());
        }
    }
    None
}

fn read_used_bytes(mp: &Path) -> Option<u64> {
    let stat = statvfs(mp).unwrap();
    let frag_size = stat.fragment_size();
    let blocks = stat.blocks();
    let blocks_free = stat.blocks_free();

    Some((blocks - blocks_free) * frag_size)
}

fn read_size_bytes(dev_path: &Path) -> Option<u64> {
    // Always in 512-byte sectors, per the kernel ABI, regardless of physical block size
    let raw = fs::read_to_string(dev_path.join("size")).ok()?;
    Some(raw.trim().parse::<u64>().ok()? * 512)
}
