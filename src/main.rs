mod combined;
mod dev_disk;
mod display;
mod fstab;
mod magic;
mod proc_mounts;
mod sys_block;

use combined::CombinedDeviceInfo;
use dev_disk::DevDiskInfo;
use fstab::FstabInfo;
use proc_mounts::ProcMountsInfo;
use sys_block::SysBlockInfo;

use tracing_subscriber::{EnvFilter, fmt};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(fmt::format::FmtSpan::CLOSE)
        .init();

    let sys_block_info = SysBlockInfo::new().ok().unwrap();
    let dev_disk_info = DevDiskInfo::new().ok().unwrap();
    let proc_mounts_info = ProcMountsInfo::new().ok().unwrap();
    let fstab_info = FstabInfo::new().ok().unwrap();

    // print!("{sys_block_info}");
    // print!("{dev_disk_info}");
    // print!("{proc_mounts_info}");

    let combined_device_info = CombinedDeviceInfo::new(
        &sys_block_info,
        &dev_disk_info,
        &proc_mounts_info,
        &fstab_info,
    );

    for device in combined_device_info {
        println!("{device}");
    }
}
