pub mod mnt_namespace;

use crate::{
    net::namespace::NetNamespace, prelude::*, process::namespace::mnt_namespace::MntNamespace,
};

pub struct Nsproxy {
    mnt_ns: Arc<MntNamespace>,
    net_ns: Arc<Mutex<NetNamespace>>,
}

impl Default for Nsproxy {
    fn default() -> Self {
        Self {
            mnt_ns: Arc::new(MntNamespace::default()),
            net_ns: NetNamespace::default(),
        }
    }
}

impl Nsproxy {
    pub fn new(mnt_ns: Arc<MntNamespace>, net_ns: Arc<Mutex<NetNamespace>>) -> Self {
        Self { mnt_ns, net_ns }
    }

    pub fn mnt_ns(&self) -> &Arc<MntNamespace> {
        &self.mnt_ns
    }

    pub fn net_ns(&self) -> &Arc<Mutex<NetNamespace>> {
        &self.net_ns
    }

    pub fn set_namespaces(&mut self, nsproxy: Arc<Mutex<Nsproxy>>) {
        let new_nsproxy = nsproxy.lock();
        self.mnt_ns = new_nsproxy.mnt_ns().clone();
        self.net_ns = new_nsproxy.net_ns().clone();
    }
}
