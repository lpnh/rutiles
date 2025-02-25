mod display;
mod sys_block;

use sys_block::SysBlockInfo;

fn main() {
    let sys_block_info = SysBlockInfo::new().ok().unwrap();

    print!("{}", sys_block_info);
}
