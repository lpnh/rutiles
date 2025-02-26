mod dev_disk;
mod display;
mod proc_mounts;
mod sys_block;

use dev_disk::DevDiskInfo;
use proc_mounts::ProcMountsInfo;
use sys_block::SysBlockInfo;

fn main() {
    let sys_block_info = SysBlockInfo::new().ok().unwrap();
    let dev_disk_info = DevDiskInfo::new().ok().unwrap();
    let proc_mounts_info = ProcMountsInfo::new().ok().unwrap();

    print!("{sys_block_info}");
    print!("{dev_disk_info}");
    print!("{proc_mounts_info}");
}
