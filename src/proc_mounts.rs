use std::fs;
use std::io::Result;

// `/proc/mounts` information abstraction for devices and partitions
// Information is obtained by parsing the file content
// Each line represents a different mount (e. g. `/dev/sdc /mnt/usb ext4 rw,relatime 0 0`)
// We only retrieve entries (mounts) that start with `/dev/`
#[derive(Debug)]
pub struct ProcMounts {
    pub name: String,        // first "field"
    pub mount_point: String, // second "field"
    pub fstype: String,      // third "field"
}

#[derive(Debug)]
pub struct ProcMountsInfo {
    pub info: Vec<ProcMounts>,
}

impl ProcMountsInfo {
    pub fn new() -> Result<Self> {
        let mut info: Vec<ProcMounts> = Vec::new();
        let mounts = fs::read_to_string("/proc/mounts")?;

        for line in mounts.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 3 {
                let dev_name = fields[0];

                if dev_name.starts_with("/dev/") {
                    let trimmed_name = dev_name
                        .strip_prefix("/dev/")
                        .expect("starts_with guaranteed");
                    let entry = ProcMounts::new(trimmed_name, fields[1], fields[2]);
                    info.push(entry);
                }
            }
        }
        Ok(Self { info })
    }
}

impl ProcMounts {
    fn new(name: &str, mount_point: &str, fstype: &str) -> Self {
        Self {
            name: name.into(),
            mount_point: mount_point.into(),
            fstype: fstype.into(),
        }
    }
}
