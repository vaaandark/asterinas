use crate::{
    fs::{rootfs::root_mount, utils::MountNode},
    prelude::*,
};

pub struct MntNamespace {
    root: Arc<MountNode>,
}

impl Default for MntNamespace {
    fn default() -> Self {
        Self {
            root: root_mount().clone(),
        }
    }
}

impl MntNamespace {
    pub fn new(mount_node: Arc<MountNode>) -> Arc<Self> {
        Arc::new(Self { root: mount_node })
    }

    pub fn root(&self) -> &Arc<MountNode> {
        &self.root
    }
}
