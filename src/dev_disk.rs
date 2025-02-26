use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::Result;

// `/dev/disk/{by-*}` information abstraction for each device and partition
// the information comes from the filename itself rather than its content
// the device name comes from the symlink target of this same file
// UUIDs are stored in an array to handle "duplicate" UUIDs from FAT filesystems
#[derive(Debug)]
pub struct DevDisk {
    pub name: String,              // e.g. "sda" or "sda1"
    pub label: Option<String>,     // from `/dev/disk/by-label` filename
    pub uuid: Option<Vec<String>>, // from `/dev/disk/by-uuid` filename
}

// Pack all the information
#[derive(Debug)]
pub struct DevDiskInfo {
    pub info: Vec<DevDisk>,
}

impl DevDiskInfo {
    pub fn new() -> Result<Self> {
        let mut labels: HashMap<OsString, String> = HashMap::new();
        let mut uuids: HashMap<OsString, Vec<String>> = HashMap::new();
        let mut device_names = HashSet::new();

        if let Ok(entries) = fs::read_dir("/dev/disk/by-label") {
            for entry in entries {
                let entry = entry?;
                if let Ok(target) = fs::read_link(entry.path()) {
                    if let Some(dev_name) = target.file_name() {
                        labels.insert(
                            dev_name.to_os_string(),
                            entry.file_name().to_string_lossy().into_owned(),
                        );
                        device_names.insert(dev_name.to_os_string());
                    }
                }
            }
        }

        if let Ok(entries) = fs::read_dir("/dev/disk/by-uuid") {
            for entry in entries {
                let entry = entry?;
                if let Ok(target) = fs::read_link(entry.path()) {
                    if let Some(dev_name) = target.file_name() {
                        let uuid_string = entry.file_name().to_string_lossy().into_owned();

                        uuids
                            .entry(dev_name.to_os_string())
                            .or_default()
                            .push(uuid_string);

                        device_names.insert(dev_name.to_os_string());
                    }
                }
            }
        }

        let mut info = Vec::new();
        for dev_name in device_names {
            let name = dev_name.to_string_lossy().into_owned();
            info.push(DevDisk {
                name,
                label: labels.remove(&dev_name),
                uuid: uuids.remove(&dev_name),
            });
        }

        Ok(Self { info })
    }
}
