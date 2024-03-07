// SPDX-License-Identifier: MPL-2.0
use crate::fs::start_block_device;
use aster_frame::vm::Vaddr;
use aster_block::BlockDevice;
use super::SyscallReturn;
use crate::{
    fs::{
        ext2::Ext2,
        fs_resolver::{FsPath, AT_FDCWD},
    },
    log_syscall_entry,
    prelude::*,
    syscall::{constants::PAGE_SIZE, SYS_MOUNT},
    util::read_cstring_from_user,
};
pub fn sys_mount(
    dev_name_addr: Vaddr,
    dir_name_addr: Vaddr,
    fs_type_name_addr: Vaddr,
    flags: u64,
    data: Vaddr,
) -> Result<SyscallReturn> {
    log_syscall_entry!(SYS_MOUNT);
    let dirname = read_cstring_from_user(dir_name_addr, PAGE_SIZE)?;
    let ext2_device_name = "vext2";
    let block_device_ext2 = start_block_device(ext2_device_name).unwrap();
    let ext2_fs = Ext2::open(block_device_ext2).unwrap();
    let current = current!();
    let target_dentry = {
        let dirname = dirname.to_string_lossy();
        let fs_path = FsPath::new(AT_FDCWD, dirname.as_ref())?;
        let dentry = current.fs().read().lookup(&fs_path)?;
        Arc::new(dentry)
    };

    target_dentry.mount(ext2_fs)?;
    Ok(SyscallReturn::Return(0))
}
