use dtmpfs::DtmpfsFactory;
use std::{boxed::Box, collections::btree_map::BTreeMap, format, sync::mutex::Mutex, vec::Vec};
use uuid::Uuid;

use crate::drivers::{
    disk::{Disk, FileSystem, FileSystemFactory, Partition},
    rfs::RfsFactory,
};

mod dtmpfs;
mod fs_tree;
mod inode;
mod operations;
mod path;
pub use inode::*;
pub use operations::*;
pub use path::*;

//0 is unknown, 1 is bad blocks, 2 is root
pub const ROOT_INODE_INDEX: u64 = 2;
pub static VFS: Mutex<Vfs> = Mutex::new(Vfs::new());

pub type DeviceId = u64;
pub type InodeIndex = u64;

pub struct DeviceDetails {
    pub drive: Uuid,
    pub partition: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InodeIdentifier {
    pub device_id: DeviceId,
    pub index: InodeIndex,
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
        }
    }

    pub fn allocate_device(&mut self) -> DeviceId {
        let id = self.device_counter;
        self.device_counter += 1;
        id
    }
}

pub fn init() {
    let mut vfs = VFS.lock();
    vfs.filesystem_driver_factories
        .insert(RfsFactory::UUID, Box::new(RfsFactory {}));
    vfs.filesystem_driver_factories
        .insert(DtmpfsFactory::UUID, Box::new(DtmpfsFactory {}));
}

pub fn resolve_path(path: &str, working_dir: &str) -> ResolvedPath {
    if path.starts_with('/') {
        resolve_single_path(path)
    } else {
        resolve_single_path(format!("{}/{}", working_dir, path).as_str())
    }
}

fn resolve_single_path(path: &str) -> ResolvedPath {
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

    ResolvedPath::new(path.into())
}
