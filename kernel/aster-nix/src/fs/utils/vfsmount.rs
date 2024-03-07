use crate::{fs::utils::Dentry, prelude::*};
pub struct Vfsmount {
    mnt_root: Arc<Dentry>,
}

impl Vfsmount {
    pub fn new(dentry: Arc<Dentry>) -> Arc<Self> {
        println!("Vfsmount::new dentry");
        Arc::new(Self { mnt_root: dentry })
    }

    pub fn mnt_root(&self) -> &Arc<Dentry> {
        &self.mnt_root
    }
}
