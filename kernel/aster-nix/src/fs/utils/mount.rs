// SPDX-License-Identifier: MPL-2.0

use super::{Dentry, DentryKey, FileSystem, InodeType, Path};
use crate::prelude::*;

/// The MountNode can form a mount tree to maintain the mount information.
pub struct MountNode {
    /// Root dentry.
    root_dentry: Arc<Dentry>,
    /// Mountpoint dentry. A mount node can be mounted on one dentry of another mount node,
    /// which makes the mount being the child of the mount node.
    mountpoint_dentry: RwLock<Option<Arc<Dentry>>>,
    /// The associated FS.
    fs: Arc<dyn FileSystem>,
    /// The parent mount.
    parent: RwLock<Option<Weak<MountNode>>>,
    /// Child mount nodes which are mounted on one dentry of self.
    children: Mutex<BTreeMap<DentryKey, Arc<Self>>>,
    /// Reference to self.
    this: Weak<Self>,
}

impl MountNode {
    /// Create a root mount node with an associated FS.
    ///
    /// The root mount node is not mounted on other mount nodes(which means it has no
    /// parent). The root inode of the fs will form the root dentry of it.
    ///
    /// It is allowed to create a mount node even if the fs has been provided to another
    /// mount node. It is the fs's responsibility to ensure the data consistency.
    pub fn new_root(fs: Arc<dyn FileSystem>) -> Arc<Self> {
        Self::new(fs, None, None)
    }

    /// The internal constructor.
    ///
    /// Root mount node has no mountpoint which other mount nodes must have mountpoint.
    fn new(
        fs: Arc<dyn FileSystem>,
        mountpoint: Option<Arc<Dentry>>,
        parent_mount: Option<Weak<MountNode>>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            root_dentry: Dentry::new_root(fs.root_inode().clone()),
            mountpoint_dentry: RwLock::new(mountpoint),
            fs,
            parent: RwLock::new(parent_mount),
            children: Mutex::new(BTreeMap::new()),
            this: weak_self.clone(),
        })
    }

    /// Clone a mount node.
    pub fn clone_mnt(
        root_dentry: Arc<Dentry>,
        mount_node: Arc<MountNode>,
        parent_mount: Option<Weak<MountNode>>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|weak_self| Self {
            root_dentry: root_dentry.clone(),
            mountpoint_dentry: RwLock::new(mount_node.mountpoint_dentry()),
            fs: mount_node.fs().clone(),
            parent: RwLock::new(parent_mount),
            children: Mutex::new(BTreeMap::new()),
            this: weak_self.clone(),
        })
    }

    /// Copy a mount tree.
    pub fn copy_tree(old_mount_node: Arc<MountNode>, old_root_dentry: Arc<Dentry>) -> Arc<Self> {
        println!("old_mount_node: {:p}", old_mount_node);
        let new_mount_node = MountNode::clone_mnt(old_root_dentry, old_mount_node.clone(), None);
        let mut stack = vec![old_mount_node.clone()];
        let mut new_stack = vec![new_mount_node.clone()];

        while let Some(current_mount_node) = stack.pop() {
            let children = current_mount_node.children.lock();
            let parent = new_stack.pop().unwrap();
            for child in children.values() {
                stack.push(child.clone());
                let new_child_mount_node = MountNode::clone_mnt(
                    child.root_dentry().clone(),
                    child.clone(),
                    Some(Arc::downgrade(&parent)),
                );
                let key = new_child_mount_node.mountpoint_dentry().unwrap().key();

                parent
                    .children
                    .lock()
                    .insert(key, new_child_mount_node.clone());
                new_stack.push(new_child_mount_node.clone());
            }
        }
        new_mount_node.clone()
    }

    pub fn attach_mnt(new_mount_node: Arc<MountNode>, new_path: Arc<Path>) {
        let parent_mount_node = new_path.mntnode();
        let mountpoint_dentry = new_path.dentry();
        let mut children = parent_mount_node.children.lock();
        let key = mountpoint_dentry.key();
        children.insert(key, new_mount_node.clone());
        new_mount_node.set_mountpoint_dentry(mountpoint_dentry.clone());
        new_mount_node.set_parent(parent_mount_node.clone());
        mountpoint_dentry.set_mountpoint();
    }

    /// Move process root and cwd directory to new mount namespace.
    pub fn process_move(mount_node: Arc<MountNode>) -> Result<()> {
        let current = current!();
        let mut stack = vec![mount_node.clone()];

        while let Some(current_mount_node) = stack.pop() {
            if Arc::ptr_eq(
                current_mount_node.root_dentry(),
                current.fs().read().root().mntnode().root_dentry(),
            ) {
                let new_root_path = Path::new(
                    current_mount_node.clone(),
                    current.fs().read().root().dentry().clone(),
                );
                current.fs().write().set_root(new_root_path.clone());
            }

            if Arc::ptr_eq(
                current_mount_node.root_dentry(),
                current.fs().read().cwd().mntnode().root_dentry(),
            ) {
                let new_cwd_path = Path::new(
                    current_mount_node.clone(),
                    current.fs().read().cwd().dentry().clone(),
                );
                current.fs().write().set_cwd(new_cwd_path.clone());
            }

            let children = current_mount_node.children.lock();
            for child in children.values() {
                stack.push(child.clone());
            }
        }
        Ok(())
    }

    /// Mount an fs on the mountpoint, it will create a new child mount node.
    ///
    /// If the given mountpoint has already been mounted, then its mounted child mount
    /// node will be updated.
    ///
    /// The mountpoint should belong to this mount node, or an error is returned.
    ///
    /// It is allowed to mount a fs even if the fs has been provided to another
    /// mountpoint. It is the fs's responsibility to ensure the data consistency.
    ///
    /// Return the mounted child mount.
    pub fn mount(&self, fs: Arc<dyn FileSystem>, mountpoint: &Arc<Path>) -> Result<Arc<Self>> {
        if !Arc::ptr_eq(mountpoint.mntnode(), &self.this()) {
            return_errno_with_message!(Errno::EINVAL, "mountpoint not belongs to this");
        }
        if mountpoint.dentry().type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }

        let key = mountpoint.dentry().key();
        let child_mount = Self::new(
            fs,
            Some(mountpoint.dentry().clone()),
            Some(Arc::downgrade(mountpoint.mntnode())),
        );
        self.children.lock().insert(key, child_mount.clone());
        Ok(child_mount)
    }

    /// Unmount a child mount node from the mountpoint and return it.
    ///
    /// The mountpoint should belong to this mount node, or an error is returned.
    pub fn umount(&self, mountpoint: &Path) -> Result<Arc<Self>> {
        if !Arc::ptr_eq(&mountpoint.mntnode(), &self.this()) {
            return_errno_with_message!(Errno::EINVAL, "mountpoint not belongs to this");
        }

        let child_mount = self
            .children
            .lock()
            .remove(&mountpoint.dentry().key())
            .ok_or_else(|| Error::with_message(Errno::ENOENT, "can not find child mount"))?;
        Ok(child_mount)
    }

    /// Try to get a child mount node from the mountpoint.
    pub fn get(&self, mountpoint: &Dentry) -> Option<Arc<Self>> {
        self.children.lock().get(&mountpoint.key()).cloned()
    }

    /// Get the root dentry of this mount node.
    pub fn root_dentry(&self) -> &Arc<Dentry> {
        &self.root_dentry
    }

    /// Try to get the mountpoint dentry of this mount node.
    pub fn mountpoint_dentry(&self) -> Option<Arc<Dentry>> {
        self.mountpoint_dentry
            .read()
            .as_ref()
            .map(|mountpoint_dentry| mountpoint_dentry.clone())
    }

    pub fn set_mountpoint_dentry(&self, mountpoint_dentry: Arc<Dentry>) {
        let mut mountpoint = self.mountpoint_dentry.write();
        *mountpoint = Some(mountpoint_dentry);
    }

    /// Flushes all pending filesystem metadata and cached file data to the device.
    pub fn sync(&self) -> Result<()> {
        let children = self.children.lock();
        for child in children.values() {
            child.sync()?;
        }
        drop(children);

        self.fs.sync()?;
        Ok(())
    }

    /// Try to get the parent mount node.
    pub fn parent(&self) -> Option<Weak<Self>> {
        self.parent
            .read()
            .as_ref()
            .map(|mount_node| mount_node.clone())
    }

    pub fn set_parent(&self, mount_node: Arc<MountNode>) {
        let mut parent = self.parent.write();
        *parent = Some(Arc::downgrade(&mount_node));
    }

    /// Get strong reference to self.
    fn this(&self) -> Arc<Self> {
        self.this.upgrade().unwrap()
    }

    /// Get the associated fs.
    pub fn fs(&self) -> &Arc<dyn FileSystem> {
        &self.fs
    }
}

impl Debug for MountNode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("MountNode")
            .field("root", &self.root_dentry)
            .field("mountpoint", &self.mountpoint_dentry)
            .field("fs", &self.fs)
            .finish()
    }
}
