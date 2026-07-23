use std::path::PathBuf;

pub enum DeviceKind {
    Disk,
    Partition { parent: String, part_num: u32 },
    Loopback { backing_file: Option<PathBuf> },
}

pub struct DiskDevice {
    pub id: String,
    pub kind: DeviceKind,
    pub fstype: Option<String>,
    pub mountpoints: Vec<PathBuf>,
    pub removable: bool,

    // In bytes
    pub size: Option<u64>,
    pub used: Option<u64>,
}
