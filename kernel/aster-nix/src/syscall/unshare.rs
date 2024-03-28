// SPDX-License-Identifier: MPL-2.0

use super::SyscallReturn;
use crate::{
    log_syscall_entry,
    prelude::*,
    process::{do_unshare, CloneFlags},
    syscall::SYS_UNSHARE,
};

pub fn sys_unshare(unshare_flags: u64) -> Result<SyscallReturn> {
    log_syscall_entry!(SYS_UNSHARE);
    debug!("flags = {:?}", unshare_flags);
    let unshare_flags: crate::process::CloneFlags = CloneFlags::from(unshare_flags);
    debug!("flags = {:?}", unshare_flags);
    let current = current!();
    println!("prepare do_unshare");
    do_unshare(unshare_flags);
    Ok(SyscallReturn::Return(0))
}
