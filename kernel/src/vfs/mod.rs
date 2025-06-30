use std::{boxed::Box, collections::btree_map::BTreeMap, format, sync::mutex::Mutex, vec::Vec};
use uuid::Uuid;

use crate::drivers::{
    disk::{Disk, FileSystem, FileSystemFactory, Partition},
    rfs::RfsFactory,
};

mod fs_tree;
mod inode;
mod operations;
pub use inode::*;
pub use operations::*;

//0 is unknown, 1 is bad blocks, 2 is root
pub const ROOT_INODE_INDEX: u32 = 2;
pub static VFS: Mutex<Vfs> = Mutex::new(Vfs::new());

///A wrapper type for path, that have been resolved to a list of path components
///That is, the path starts from root and does not contain any "." or ".." components
#[repr(transparent)]
pub struct ResolvedPath(Box<[Box<str>]>);

//just a wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DeviceId(u64);

pub struct DeviceDetails {
    pub drive: Uuid,
    pub partition: Uuid,
}

pub struct Vfs {
    ///Map from disk guid to disk object (driver) and a list of partition guids
    disks: BTreeMap<Uuid, (Box<dyn Disk + Send>, Vec<Uuid>)>,
    ///maps from filesystem type guid to filesystem driver factory
    filesystem_driver_factories: BTreeMap<Uuid, Box<dyn FileSystemFactory + Send>>,
    ///maps from partition guid to filesystem driver
    mounted_partitions: BTreeMap<Uuid, Box<dyn FileSystem + Send>>,
    ///maps from partition guid to partition object
    available_partitions: BTreeMap<Uuid, Partition>,
    ///maps from device id to partition and drive uuid
    devices: BTreeMap<DeviceId, DeviceDetails>,
    ///counts devices
    device_counter: u64,
    ///map from path to filesystem uuid
    mount_points: BTreeMap<Box<[Box<str>]>, Uuid>,
}

impl Vfs {
    const fn new() -> Self {
        Vfs {
            disks: BTreeMap::new(),
            filesystem_driver_factories: BTreeMap::new(),
            mounted_partitions: BTreeMap::new(),
            available_partitions: BTreeMap::new(),
            devices: BTreeMap::new(),
            device_counter: 0,
            mount_points: BTreeMap::new(),
        }
    }

    pub fn allocate_device(&mut self) -> DeviceId {
        let id = DeviceId(self.device_counter);
        self.device_counter += 1;
        id
    }
}

pub fn init() {
    VFS.lock()
        .filesystem_driver_factories
        .insert(RfsFactory::guid(), Box::new(RfsFactory {}));
}

pub fn resolve_path(path: &str, working_dir: &str) -> ResolvedPath {
    if path.starts_with('/') {
        ResolvedPath(resolve_single_path(path))
    } else {
        let resolved_path = resolve_single_path(format!("{}/{}", working_dir, path).as_str());
        ResolvedPath(resolved_path)
    }
}

fn resolve_single_path(path: &str) -> Box<[Box<str>]> {
    let chunks = path.split('/');
    let mut path = Vec::new();
    for chunk in chunks {
        if chunk.is_empty() {
            continue;
        }
        if chunk == "." {
            continue;
        }
        if chunk == ".." {
            path.pop();
            continue;
        }
        path.push(chunk.into());
    }

    path.into()
}
