use std::fmt;

use super::dev_disk::DevDiskInfo;
use super::proc_mounts::ProcMountsInfo;
use super::sys_block::SysBlockInfo;

impl fmt::Display for SysBlockInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation
        )]
        fn readable_size_from(size: u64) -> String {
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
                // handle trailing zeros
                format!("{:.0}{}", size, UNITS[unit_index])
            } else {
                format!("{:.1}{}", size, UNITS[unit_index])
            }
        }

        writeln!(f)?; // Extra line
        writeln!(f, "from `/sys/block`")?;
        writeln!(f, "=================")?;
        for device in &self.info {
            writeln!(f)?; // Extra line
            writeln!(f, "⛊ {}", device.name)?;
            writeln!(f, " • Model: {}", device.info.model)?;
            writeln!(f, " • Size: {}", readable_size_from(device.info.size))?;
            writeln!(
                f,
                " • Removable: {}",
                if device.info.removable { "Yes" } else { "No" }
            )?;

            // List partitions, if some
            if let Some(parts) = &device.part {
                writeln!(f, " • Partitions:")?;
                for part in parts {
                    writeln!(f, "    ⛉ {}", part.name)?;
                    writeln!(f, "      • Size: {}", readable_size_from(part.info.size))?;
                    writeln!(
                        f,
                        "      • Removable: {}",
                        if part.info.removable { "Yes" } else { "No" }
                    )?;
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for DevDiskInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?; // Extra line
        writeln!(f, "from `/dev/disk`")?;
        writeln!(f, "================")?;

        for device in &self.info {
            writeln!(f)?; // Extra line
            writeln!(f, "⛉ {}", device.name)?;

            if let Some(label) = &device.label {
                writeln!(f, "  • Label: {label}")?;
            }

            if let Some(uuid_vec) = &device.uuid {
                for uuid in uuid_vec {
                    writeln!(f, "  • UUID: {uuid}")?;
                }
            }
        }

        Ok(())
    }
}

impl fmt::Display for ProcMountsInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?; // Extra line
        writeln!(f, "from `/proc/mounts`")?;
        writeln!(f, "===================")?;

        for device in &self.info {
            writeln!(f)?; // Extra line
            writeln!(f, "⛉ {}", device.name)?;

            let fstype = &device.fstype;
            writeln!(f, "  • Filesystem: {fstype}")?;

            let mount_point = &device.mount_point;
            writeln!(f, "  • Mount Point: {mount_point}")?;
        }

        Ok(())
    }
}
