use std::fmt;

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
            writeln!(f, " {}", device.name)?;
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
                    writeln!(f, "     {}", part.name)?;
                    writeln!(f, "      • Size: {}", readable_size_from(part.info.size))?;
                }
            }
        }
        Ok(())
    }
}
