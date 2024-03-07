pub mod mnt_namespace;

use crate::{prelude::*, process::namespace::mnt_namespace::MntNamespace};

pub struct Nsproxy {
    mnt_ns: Arc<MntNamespace>,
}

impl Default for Nsproxy {
    fn default() -> Self {
        Self {
            mnt_ns: Arc::new(MntNamespace::default()),
        }
    }
}

impl Nsproxy {
    pub fn new(mntnamespace: Arc<MntNamespace>) -> Self {
        Self {
            mnt_ns: mntnamespace,
        }
    }

    pub fn mnt_ns(&self) -> &Arc<MntNamespace> {
        &self.mnt_ns
    }
}
