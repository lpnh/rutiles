use std::fs::File;
use std::io::{Error, Read, Seek, SeekFrom};
use tracing::{info, warn};

// Filesystem magic numbers
// source: <https://github.com/torvalds/linux/blob/master/include/uapi/linux/magic.h>
// another source: <https://github.com/file/file/blob/master/magic/Magdir/filesystems>
pub mod fs_magic {
    pub const EXT4_MAGIC: u16 = 0xEF53;
    pub const BTRFS_MAGIC: u64 = 0x9123_683E;
    pub const XFS_MAGIC: u32 = 0x5846_5342;
    pub const NTFS_MAGIC: &[u8] = b"NTFS ";
    pub const FAT12_MAGIC: &[u8] = b"FAT12   ";
    pub const FAT16_MAGIC: &[u8] = b"FAT16   ";
    pub const FAT32_MAGIC: &[u8] = b"FAT32   ";
    pub const EXFAT_MAGIC: &[u8] = b"EXFAT   ";
    pub const SWAP_MAGIC: &[u8] = b"SWAP-SPACE";
    pub const SWAP_MAGIC_2: &[u8] = b"SWAPSPACE2";
    pub const ISO9660_MAGIC: &[u8] = b"CD001";
}

#[derive(Debug, Clone, PartialEq)]
pub enum FsType {
    Ext4,
    Btrfs,
    Xfs,
    Ntfs,
    Vfat,
    ExFat,
    Swap,
    Iso9660,
}

enum Detection {
    // Look for a byte sequence at a specified offset
    ByteSequence {
        offset: u64,
        signature: &'static [u8],
        secondary_check: Option<fn(&[u8]) -> bool>,
    },
    // Look for a {n}-bit magic number at a specified offset
    MagicU16 {
        offset: u64,
        magic: u16,
    },
    MagicU32 {
        offset: u64,
        magic: u32,
    },
    MagicU64 {
        offset: u64,
        magic: u64,
    },
}

struct Signature {
    method: Detection,
    fs_type: FsType,
}

// Get filesystem type using magic numbers
// It seems to require root privileges ☹
#[tracing::instrument]
pub fn get_fstype_with_magic(device: &str) -> Result<Option<String>, Error> {
    let path = format!("/dev/{device}");
    let mut file = File::open(&path)?;

    let signatures = vec![
        Signature {
            fs_type: FsType::Vfat, // FAT12
            method: Detection::ByteSequence {
                offset: 0x36,
                signature: fs_magic::FAT12_MAGIC,
                secondary_check: Some(has_boot_sector),
            },
        },
        Signature {
            fs_type: FsType::Vfat, // FAT16
            method: Detection::ByteSequence {
                offset: 0x36,
                signature: fs_magic::FAT16_MAGIC,
                secondary_check: Some(has_boot_sector),
            },
        },
        Signature {
            fs_type: FsType::Vfat, // FAT32
            method: Detection::ByteSequence {
                offset: 0x52,
                signature: fs_magic::FAT32_MAGIC,
                secondary_check: Some(has_boot_sector),
            },
        },
        Signature {
            fs_type: FsType::Ntfs,
            method: Detection::ByteSequence {
                offset: 3,
                signature: fs_magic::NTFS_MAGIC,
                secondary_check: Some(has_boot_sector),
            },
        },
        Signature {
            fs_type: FsType::ExFat,
            method: Detection::ByteSequence {
                offset: 3,
                signature: fs_magic::EXFAT_MAGIC,
                secondary_check: None,
            },
        },
        Signature {
            fs_type: FsType::Swap,
            method: Detection::ByteSequence {
                offset: 4096 - 10,                 // at end of the first 4096 bytes
                signature: fs_magic::SWAP_MAGIC_2, // SWAPSPACE2
                secondary_check: None,
            },
        },
        Signature {
            fs_type: FsType::Swap,
            method: Detection::ByteSequence {
                offset: 4096 - 10,
                signature: fs_magic::SWAP_MAGIC, // SWAP-SPACE
                secondary_check: None,
            },
        },
        Signature {
            fs_type: FsType::Xfs, // XFS
            method: Detection::MagicU32 {
                offset: 0,
                magic: fs_magic::XFS_MAGIC,
            },
        },
        Signature {
            fs_type: FsType::Ext4,
            method: Detection::MagicU16 {
                offset: 1080, // Offset 0x438
                magic: fs_magic::EXT4_MAGIC,
            },
        },
        Signature {
            fs_type: FsType::Iso9660,
            method: Detection::ByteSequence {
                offset: 0x8001, // Sector 16 + 1
                signature: fs_magic::ISO9660_MAGIC,
                secondary_check: None,
            },
        },
        Signature {
            fs_type: FsType::Iso9660,
            method: Detection::ByteSequence {
                offset: 0x8801, // Sector 17 + 1
                signature: fs_magic::ISO9660_MAGIC,
                secondary_check: None,
            },
        },
        Signature {
            fs_type: FsType::Iso9660,
            method: Detection::ByteSequence {
                offset: 0x9001, // Sector 18 + 1
                signature: fs_magic::ISO9660_MAGIC,
                secondary_check: None,
            },
        },
        Signature {
            fs_type: FsType::Btrfs,
            method: Detection::MagicU64 {
                offset: 65600, // 64K + 64 bytes
                magic: fs_magic::BTRFS_MAGIC,
            },
        },
    ];

    for sig in &signatures {
        match &sig.method {
            Detection::ByteSequence {
                offset,
                signature,
                secondary_check,
            } => {
                // Read enough data to check the signature
                let read_size = (*offset as usize + signature.len()).max(4096);
                let mut buffer = vec![0u8; read_size];

                // Seek to the beginning and read the necessary data
                if file.seek(SeekFrom::Start(0)).is_err() {
                    warn!("Failed to seek to beginning of file");
                    continue;
                }

                if let Err(e) = file.read_exact(&mut buffer) {
                    warn!(%e, "Failed to read {} bytes", read_size);
                    continue;
                }

                // Check boot sector signature
                if let Some(check_fn) = secondary_check {
                    if !check_fn(&buffer) {
                        continue;
                    }
                }

                // Check the signature itself
                let end_offset = *offset as usize + signature.len();
                if buffer[*offset as usize..end_offset] == **signature {
                    info!("Detected signature for {:#?}", sig.fs_type);
                    return Ok(Some(fs_type_to_string(&sig.fs_type)));
                }
            }
            Detection::MagicU16 { offset, magic } => {
                if file.seek(SeekFrom::Start(*offset)).is_err() {
                    warn!("Failed to seek to position {}", offset);
                    continue;
                }

                let mut magic_bytes = [0u8; 2];
                if file.read_exact(&mut magic_bytes).is_err() {
                    warn!("Failed to read magic number at offset {}", offset);
                    continue;
                }

                let value = u16::from_le_bytes(magic_bytes);
                if value == *magic {
                    info!("Detected signature for {:#?}", sig.fs_type);
                    return Ok(Some(fs_type_to_string(&sig.fs_type)));
                }
            }
            Detection::MagicU32 { offset, magic } => {
                if file.seek(SeekFrom::Start(*offset)).is_err() {
                    warn!("Failed to seek to position {}", offset);
                    continue;
                }

                let mut magic_bytes = [0u8; 4];
                if file.read_exact(&mut magic_bytes).is_err() {
                    warn!("Failed to read magic number at offset {}", offset);
                    continue;
                }

                let value = u32::from_le_bytes(magic_bytes);
                if value == *magic {
                    info!("Detected signature for {:#?}", sig.fs_type);
                    return Ok(Some(fs_type_to_string(&sig.fs_type)));
                }
            }
            Detection::MagicU64 { offset, magic } => {
                if file.seek(SeekFrom::Start(*offset)).is_err() {
                    warn!("Failed to seek to position {}", offset);
                    continue;
                }

                let mut magic_bytes = [0u8; 8];
                if file.read_exact(&mut magic_bytes).is_err() {
                    warn!("Failed to read magic number at offset {}", offset);
                    continue;
                }

                let value = u64::from_le_bytes(magic_bytes);
                if value == *magic {
                    info!("Detected signature for {:#?}", sig.fs_type);
                    return Ok(Some(fs_type_to_string(&sig.fs_type)));
                }
            }
        }
    }

    warn!("Could not determine fs type for `{device}`");
    Ok(None)
}

// Check boot sector signature at 510-511
fn has_boot_sector(buffer: &[u8]) -> bool {
    buffer.len() >= 512 && buffer[510] == 0x55 && buffer[511] == 0xAA
}

fn fs_type_to_string(fs_type: &FsType) -> String {
    match fs_type {
        FsType::ExFat => "exfat".to_string(),
        FsType::Btrfs => "btrfs".to_string(),
        FsType::Xfs => "xfs".to_string(),
        FsType::Ntfs => "ntfs".to_string(),
        FsType::Vfat => "vfat".to_string(),
        FsType::Ext4 => "ext4".to_string(),
        FsType::Swap => "swap".to_string(),
        FsType::Iso9660 => "iso9660".to_string(),
    }
}
