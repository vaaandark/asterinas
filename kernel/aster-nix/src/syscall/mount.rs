// SPDX-License-Identifier: MPL-2.0
use aster_frame::vm::Vaddr;

use super::SyscallReturn;
use crate::{
    fs::{
        ext2::Ext2,
        fs_resolver::{FsPath, AT_FDCWD},
        start_block_device,
        utils::{MountNode, Path},
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

    let devname = read_cstring_from_user(dev_name_addr, PAGE_SIZE)?;
    let dirname = read_cstring_from_user(dir_name_addr, PAGE_SIZE)?;

    let mount_flags = MountFlags::from_bits_truncate(flags as u32);

    let current = current!();
    let target_path = {
        let dirname = dirname.to_string_lossy();
        if dirname.is_empty() {
            return_errno_with_message!(Errno::ENOENT, "dirname is empty");
        }
        let fs_path = FsPath::new(AT_FDCWD, dirname.as_ref())?;
        let path = current.fs().read().lookup(&fs_path)?;
        path
    };

    if mount_flags.contains(MountFlags::MS_REMOUNT) && mount_flags.contains(MountFlags::MS_BIND) {
        do_reconfigure_mnt();
    } else if mount_flags.contains(MountFlags::MS_REMOUNT) {
        do_remount();
    } else if mount_flags.contains(MountFlags::MS_BIND) {
        do_loopback(devname.clone(), target_path.clone());
    } else if mount_flags.contains(MountFlags::MS_SHARED)
        | mount_flags.contains(MountFlags::MS_PRIVATE)
        | mount_flags.contains(MountFlags::MS_SLAVE)
        | mount_flags.contains(MountFlags::MS_UNBINDABLE)
    {
        do_change_type();
    } else if mount_flags.contains(MountFlags::MS_MOVE) {
        do_move_mount_old(devname, target_path);
    } else {
        do_new_mount(devname, target_path);
    }

    Ok(SyscallReturn::Return(0))
}

fn do_reconfigure_mnt() {
    // TODO
}

fn do_remount() {
    // TODO
}

fn do_loopback(old_name: CString, new_path: Arc<Path>) -> Result<()> {
    let current = current!();
    let old_path = {
        let old_name = old_name.to_string_lossy();
        if old_name.is_empty() {
            return_errno_with_message!(Errno::ENOENT, "old_name is empty");
        }
        let fs_path = FsPath::new(AT_FDCWD, old_name.as_ref())?;
        let path = current.fs().read().lookup(&fs_path)?;
        path
    };

    let new_mount_node =
        MountNode::copy_tree(old_path.mntnode().clone(), old_path.dentry().clone());

    MountNode::attach_mnt(new_mount_node.clone(), new_path.clone());
    Ok(())
}

fn do_change_type() {
    // TODO
}

fn do_move_mount_old(old_name: CString, new_path: Arc<Path>) -> Result<()> {
    let current = current!();
    let old_path = {
        let old_name = old_name.to_string_lossy();
        if old_name.is_empty() {
            return_errno_with_message!(Errno::ENOENT, "old_name is empty");
        }
        let fs_path = FsPath::new(AT_FDCWD, old_name.as_ref())?;
        let path = current.fs().read().lookup(&fs_path)?;
        path
    };

    if !old_path.dentry().is_root_of_mount() && old_path.mntnode().parent().is_none() {
        return_errno_with_message!(Errno::EINVAL, "old_name is not a mountpoint");
    }

    MountNode::unattch_mnt(old_path.mntnode().clone());
    MountNode::attach_mnt(old_path.mntnode().clone(), new_path.clone());

    Ok(())
}

fn do_new_mount(devname: CString, target_path: Arc<Path>) -> Result<()> {
    let ext2_device_name = "vext2";
    let block_device_ext2 = start_block_device(ext2_device_name).unwrap();
    let ext2_fs = Ext2::open(block_device_ext2).unwrap();

    target_path.mount(ext2_fs)?;
    Ok(())
}

bitflags! {
    struct MountFlags: u32 {
        const MS_RDONLY = 1;              /* Mount read-only */
        const MS_NOSUID = 1<<1;           /* Ignore suid and sgid bits */
        const MS_NODEV = 1<<2;            /* Disallow access to device special files */
        const MS_NOEXEC = 1<<3;           /* Disallow program execution */
        const MS_SYNCHRONOUS = 1<<4;      /* Writes are synced at once */
        const MS_REMOUNT = 1<<5;          /* Alter flags of a mounted FS */
        const MS_MANDLOCK = 1<<6;         /* Allow mandatory locks on an FS */
        const MS_DIRSYNS = 1<<7;          /* Directory modifications are synchronous */
        const MS_NOSYMFOLLOW = 1<<8;      /* Do not follow symlinks */
        const MS_NOATIME = 1<<10;         /* Do not update access times. */
        const MS_NODIRATIME = 1<<11;      /* Do not update directory access times */
        const MS_BIND = 1<<12;
        const MS_MOVE = 1<<13;
        const MS_REC = 1<<14;
        const MS_VERBOSE = 1<<15;         /* War is peace. Verbosity is silence. MS_VERBOSE is deprecated. */

        const MS_SILENT = 1<<15;
        const MS_POSIXACL = 1<<16;        /* VFS does not apply the umask */
        const MS_UNBINDABLE = 1<<17;      /* change to unbindable */
        const MS_PRIVATE = 1<<18; 	      /* change to private */
        const MS_SLAVE = 1<<19;           /* change to slave */
        const MS_SHARED = 1<<20;          /* change to shared */
        const MS_RELATIME = 1<<21; 	      /* Update atime relative to mtime/ctime. */
        const MS_KERNMOUNT = 1<<22;       /* this is a kern_mount call */
    }
}
