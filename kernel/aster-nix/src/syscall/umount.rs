// SPDX-License-Identifier: MPL-2.0
use aster_frame::vm::Vaddr;

use super::SyscallReturn;
use crate::{
    fs::{
        fs_resolver::{FsPath, AT_FDCWD},
        utils::{Path, PATH_MAX},
    },
    log_syscall_entry,
    prelude::*,
    syscall::SYS_UMOUNT,
    util::read_cstring_from_user,
};
pub fn sys_umount(pathname: Vaddr, flags: u64) -> Result<SyscallReturn> {
    log_syscall_entry!(SYS_UMOUNT);

    let pathname = read_cstring_from_user(pathname, PATH_MAX)?;
    let umount_flags = UmountFlags::from_bits_truncate(flags as u32);
    debug!("pathname = {:?}, flags = {:?}", pathname, umount_flags);

    umount_flags.check_unsupported_flags();

    let current = current!();
    let pathname = pathname.to_string_lossy();
    let fs_path = FsPath::new(AT_FDCWD, pathname.as_ref())?;

    let umount_path = if umount_flags.contains(UmountFlags::UMOUNT_NOFOLLOW) {
        let path = current.fs().read().lookup_no_follow(&fs_path)?;
        path
    } else {
        let path = current.fs().read().lookup(&fs_path)?;
        path
    };

    umount_path.umount()?;

    Ok(SyscallReturn::Return(0))
}

bitflags! {
    struct UmountFlags: u32 {
        const MNT_FORCE = 0x00000001;	/* Attempt to forcibily umount */
        const MNT_DETACH = 0x00000002;	/* Just detach from the tree */
        const MNT_EXPIRE = 0x00000004;	/* Mark for expiry */
        const UMOUNT_NOFOLLOW = 0x00000008;	/* Don't follow symlink on umount */
    }
}

impl UmountFlags {
    fn check_unsupported_flags(&self) -> Result<()> {
        let supported_flags = UmountFlags::MNT_FORCE
            | UmountFlags::MNT_DETACH
            | UmountFlags::MNT_EXPIRE
            | UmountFlags::UMOUNT_NOFOLLOW;
        let unsupported_flags = *self - supported_flags;
        if !unsupported_flags.is_empty() {
            return_errno_with_message!(Errno::EINVAL, "unsupported flags");
        }
        Ok(())
    }
}
