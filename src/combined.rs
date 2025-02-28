use std::fmt;
use tracing::warn;

use super::dev_disk::DevDiskInfo;
use super::fstab::{Fstab, FstabInfo};
use super::magic::get_fstype_with_magic;
use super::proc_mounts::ProcMountsInfo;
use super::sys_block::SysBlockInfo;

#[derive(Debug)]
pub struct CombinedPartitionInfo {
    pub name: String,
    pub size: Option<u64>,
    pub filesystem: Option<String>,
    pub label: Option<String>,
    pub mount_point: Option<String>,
    pub removable: Option<bool>,
    pub uuids: Option<Vec<String>>,
    pub fstab_entry: Option<Fstab>,
}

#[derive(Debug)]
pub struct CombinedDeviceInfo {
    pub name: String,
    pub model: Option<String>,
    pub size: Option<u64>,
    pub filesystem: Option<String>,
    pub label: Option<String>,
    pub mount_point: Option<String>,
    pub removable: Option<bool>,
    pub uuids: Option<Vec<String>>,
    pub fstab_entry: Option<Fstab>,
    pub partitions: Vec<CombinedPartitionInfo>,
}

impl CombinedDeviceInfo {
    pub fn new(
        sys_block: &SysBlockInfo,
        dev_disk: &DevDiskInfo,
        proc_mounts: &ProcMountsInfo,
        fstab: &FstabInfo,
    ) -> Vec<Self> {
        let mut combined_info = Vec::new();

        // Start with the information from `/sys/block`
        for sys_block in &sys_block.info {
            let mut combined_device = Self {
                name: sys_block.name.clone(),
                model: Some(sys_block.info.model.clone()),
                size: Some(sys_block.info.size),
                filesystem: None,
                label: None,
                mount_point: None,
                removable: Some(sys_block.info.removable),
                uuids: None,
                fstab_entry: None,
                partitions: Vec::new(),
            };

            // Add information from `/dev/disk`
            if let Some(dev_disk) = dev_disk.info.iter().find(|d| d.name == sys_block.name) {
                combined_device.label.clone_from(&dev_disk.label);
                combined_device.uuids.clone_from(&dev_disk.uuid);
            }

            // Add information from `/proc/mounts`
            if let Some(proc_mounts) = proc_mounts.info.iter().find(|d| d.name == sys_block.name) {
                combined_device.mount_point = Some(proc_mounts.mount_point.clone());
                combined_device.filesystem = Some(proc_mounts.fstype.clone());
            }

            // Fallback to magic numbers to find filesystem type
            if combined_device.filesystem.is_none() && sys_block.part.is_none()
            // && is_running_with_sudo()
            {
                combined_device.filesystem = match get_fstype_with_magic(&sys_block.name) {
                    Ok(fs_type) => fs_type,
                    Err(e) => {
                        warn!("Failed to get fstype from signature: {e}");
                        None
                    }
                }
            }

            // Try to find a matching fstab entry
            let get_dev_fstab_entry = fstab.info.iter().find(|entry| {
                combined_device // by UUID
                    .uuids
                    .as_ref()
                    .and_then(|uuids| uuids.first())
                    .map(|uuid| format!("UUID={}", uuid) == entry.device)
                    .unwrap_or(false)
                    || combined_device // by label
                        .label
                        .as_ref()
                        .map(|label| format!("LABEL={}", label) == entry.device)
                        .unwrap_or(false)
            });
            if let Some(fstab_entry) = get_dev_fstab_entry {
                combined_device.fstab_entry = Some(fstab_entry.clone());
            }

            // Same thing for partitions...
            if let Some(parts) = &sys_block.part {
                for part in parts {
                    let mut combined_partition = CombinedPartitionInfo {
                        name: part.name.clone(),
                        size: Some(part.info.size),
                        filesystem: None,
                        label: None,
                        mount_point: None,
                        removable: Some(part.info.removable),
                        uuids: None,
                        fstab_entry: None,
                    };

                    if let Some(dev_part) = dev_disk.info.iter().find(|d| d.name == part.name) {
                        combined_partition.label.clone_from(&dev_part.label);
                        combined_partition.uuids.clone_from(&dev_part.uuid);
                    }

                    if let Some(proc_part) = proc_mounts.info.iter().find(|d| d.name == part.name) {
                        combined_partition.mount_point = Some(proc_part.mount_point.clone());
                        combined_partition.filesystem = Some(proc_part.fstype.clone());
                    }

                    let get_part_fstab_entry = fstab.info.iter().find(|entry| {
                        combined_partition // by UUID
                            .uuids
                            .as_ref()
                            .and_then(|uuids| uuids.first())
                            .map(|uuid| format!("UUID={}", uuid) == entry.device)
                            .unwrap_or(false)
                            || combined_partition // by label
                                .label
                                .as_ref()
                                .map(|label| format!("LABEL={}", label) == entry.device)
                                .unwrap_or(false)
                    });
                    if let Some(fstab_entry) = get_part_fstab_entry {
                        combined_partition.fstab_entry = Some(fstab_entry.clone());
                    }

                    if combined_partition.filesystem.is_none() {
                        combined_partition.filesystem = match get_fstype_with_magic(&part.name) {
                            Ok(fs_type) => fs_type,
                            Err(e) => {
                                warn!("Failed to get fstype from signature: {e}");
                                None
                            }
                        }
                    }

                    combined_device.partitions.push(combined_partition);
                }
            }

            // Sort partitions
            combined_device
                .partitions
                .sort_by(|a, b| a.name.cmp(&b.name));
            combined_info.push(combined_device);
        }

        // Sort devices
        combined_info.sort_by(|a, b| a.name.cmp(&b.name));
        combined_info
    }
}

impl fmt::Display for CombinedDeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "⛊ {}", self.name)?;
        let indent = "  "; // 2 spaces for indentation

        // Device-specific field
        if let Some(model) = &self.model {
            writeln!(f, "{indent}• Model: {model}")?;
        }

        // Common fields
        format_common_fields(
            f,
            indent,
            self.size,
            self.filesystem.as_ref(),
            self.label.as_ref(),
            self.mount_point.as_ref(),
            self.removable,
            self.uuids.as_ref(),
            self.fstab_entry.as_ref(),
        )?;

        // Partition section
        if !self.partitions.is_empty() {
            writeln!(f, "{indent}• Partitions:")?;
            for partition in &self.partitions {
                write!(f, "    ")?; // 4 spaces for indentation
                partition.fmt(f)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for CombinedPartitionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "⛉ {}", self.name)?;
        let indent = "      "; // 6 spaces for indentation

        format_common_fields(
            f,
            indent,
            self.size,
            self.filesystem.as_ref(),
            self.label.as_ref(),
            self.mount_point.as_ref(),
            self.removable,
            self.uuids.as_ref(),
            self.fstab_entry.as_ref(),
        )?;

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn format_common_fields(
    f: &mut fmt::Formatter<'_>,
    indent: &str,
    size: Option<u64>,
    filesystem: Option<&String>,
    label: Option<&String>,
    mount_point: Option<&String>,
    removable: Option<bool>,
    uuids: Option<&Vec<String>>,
    fstab_entry: Option<&Fstab>,
) -> fmt::Result {
    if let Some(size) = size {
        writeln!(f, "{indent}• Size: {}", readable_size_from(size))?;
    }
    if let Some(filesystem) = filesystem {
        writeln!(f, "{indent}• Filesystem: {filesystem}")?;
    }
    if let Some(label) = label {
        writeln!(f, "{indent}• Label: {label}")?;
    }
    if let Some(mount_point) = mount_point {
        writeln!(f, "{indent}• Mount Point: {mount_point}")?;
    }
    if let Some(removable) = removable {
        writeln!(
            f,
            "{}• Removable: {}",
            indent,
            if removable { "Yes" } else { "No" }
        )?;
    }
    if let Some(uuids) = uuids {
        if uuids.len() == 1 {
            writeln!(f, "{indent}• UUID: {}", uuids[0])?;
        } else {
            // For FAT filesystems
            writeln!(f, "{indent}• UUID: {} ({})", uuids[0], uuids[1])?;
        }
    }
    let extra_indent = "  ";
    if let Some(fstab_entry) = fstab_entry {
        writeln!(f, "{indent}• Fstab Entry:")?;
        writeln!(f, "{indent}{extra_indent}• Device: {}", fstab_entry.device)?;
        writeln!(
            f,
            "{indent}{extra_indent}• Mount Point: {}",
            fstab_entry.mount_point
        )?;
        writeln!(
            f,
            "{indent}{extra_indent}• Filesystem: {}",
            fstab_entry.fs_type
        )?;
        writeln!(f, "{indent}{extra_indent}• Options:")?;
        for option in &fstab_entry.options {
            writeln!(f, "{indent}{extra_indent}{extra_indent}• {option}")?;
        }
        writeln!(
            f,
            "{indent}{extra_indent}• Dump Frequency: {}",
            fstab_entry.dump_freq
        )?;
        writeln!(
            f,
            "{indent}{extra_indent}• fsck Pass: {}",
            fstab_entry.fsck_pass
        )?;
    }

    Ok(())
}

fn readable_size_from(size: u64) -> String {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]
    {
        const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{}{}", size as u64, UNITS[unit_index])
        } else if size.fract() == 0.0 {
            format!("{:.0}{}", size, UNITS[unit_index])
        } else {
            format!("{:.1}{}", size, UNITS[unit_index])
        }
    }
}
