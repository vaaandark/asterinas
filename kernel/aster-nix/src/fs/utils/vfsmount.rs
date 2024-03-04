use super::{dentry, SuperBlock};
use crate::{fs::utils::Dentry, prelude::*};
pub struct Vfsmount {
    mnt_root: Arc<Dentry>,
}

impl Vfsmount {
    pub fn new(dentry: Arc<Dentry>) -> Arc<Self> {
        Arc::new(Self { mnt_root: dentry })
    }
}
