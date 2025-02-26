use std::{
    fs,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
};

// `/sys/block/` entries, stored in an array
//    Each symlink `PathBuf` represents a device
#[derive(Debug)]
pub struct SysBlockEntries {
    pub block_devices: Vec<PathBuf>, // e.g. `["/sys/block/nvme0n1", "/sys/block/sda"]`
}

// `/sys/block/{device}/` entries
//    note: any partition will appear as an entry here (e.g. `sda1/`)
#[derive(Debug)]
pub struct SysBlockDeviceEntries {
    pub model: String,
    pub removable: bool,
    pub size: u64,
}

// `/sys/block/{device}/{partition}/` entries
#[derive(Debug)]
pub struct SysBlockPartitionEntries {
    pub size: u64, // It seems `size` is the only relevant information
}

// Information abstraction for each partition
#[derive(Debug)]
pub struct SysBlockPartition {
    pub name: String,                   // e.g. `"sda1"`
    pub info: SysBlockPartitionEntries, // For now, only its size
}

// Information abstraction for each device
#[derive(Debug)]
pub struct SysBlockDevice {
    pub name: String, // e.g. `"sda"`
    pub info: SysBlockDeviceEntries,
    pub part: Option<Vec<SysBlockPartition>>, // All the partitions and their own information
}

// Pack all the information
#[derive(Debug)]
pub struct SysBlockInfo {
    pub info: Vec<SysBlockDevice>,
}

impl SysBlockInfo {
    pub fn new() -> Result<Self> {
        let block_devices = SysBlockEntries::new()?;
        let mut info = Vec::new();

        for device_path in &block_devices.block_devices {
            info.push(SysBlockDevice::new(device_path)?);
        }

        Ok(Self { info })
    }
}

impl SysBlockEntries {
    pub fn new() -> Result<Self> {
        let mut block_devices = Vec::<PathBuf>::new();

        for entry in fs::read_dir("/sys/block")? {
            let device_name: PathBuf = entry?.path();
            block_devices.push(device_name);
        }

        Ok(Self { block_devices })
    }
}

impl SysBlockDevice {
    pub fn new(block_device: &Path) -> Result<Self> {
        // Extract device name from Path
        let name = block_device
            .file_name()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid device path"))?
            .to_string_lossy()
            .to_string();

        // Create partition array from `/sys/block/{device}` entries
        let partition = fs::read_dir(block_device)?
            .filter_map(Result::ok)
            // Into String... So we can use `starts_with`
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|entry| entry.starts_with(&name))
            .map(|part_name| SysBlockPartition::new(block_device, &part_name))
            .collect::<Result<Vec<SysBlockPartition>>>()?;

        let part = if partition.is_empty() {
            None
        } else {
            Some(partition)
        };

        let size = read_size(block_device)?;
        let removable = read_removable(block_device)?;
        let model = read_device_model(block_device)?;

        let info = SysBlockDeviceEntries {
            model,     // from `/sys/block/{device}/device/model`
            removable, // from `/sys/block/{device}/removable`
            size,      // from `/sys/block/{device}/size`
        };

        Ok(Self { name, info, part })
    }
}

impl SysBlockPartition {
    fn new(dev_path: &Path, part_name: &str) -> Result<Self> {
        Ok(Self {
            name: part_name.to_string(),
            info: SysBlockPartitionEntries::new(dev_path, part_name)?,
        })
    }
}

impl SysBlockPartitionEntries {
    fn new(dev_path: &Path, part_name: &str) -> Result<Self> {
        let size = read_size(&dev_path.join(part_name))?;
        Ok(Self {
            size, // from `/sys/block/{device}/{partition}/size`
        })
    }
}

fn read_size(path: &Path) -> Result<u64> {
    let size_str = fs::read_to_string(path.join("size"))?;
    size_str
        .trim()
        .parse::<u64>()
        .map(|blocks| blocks * 512) // Convert 512-byte blocks to bytes
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

fn read_removable(path: &Path) -> Result<bool> {
    let removable_str = fs::read_to_string(path.join("removable"))?;
    Ok(removable_str.trim() == "1") // unknown -1, yes 1, not 0
}

fn read_device_model(path: &Path) -> Result<String> {
    let model_str = fs::read_to_string(path.join("device/model"))?;
    Ok(model_str.trim().to_string())
}
