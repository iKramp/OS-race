use dtmpfs::DtmpfsFactory;
use std::{sync::arc::Arc, boxed::Box, collections::btree_map::BTreeMap, lock_w_info, sync::{lock_info::LockLocationInfo, no_int_spinlock::NoIntSpinlock}, vec::Vec};
use uuid::Uuid;

use crate::drivers::{
    disk::{BlockDevice, Partition},
    rfs::RfsFactory,
};

mod adapters;
mod dtmpfs;
mod filesystem_trait;
mod fs_tree;
mod inode;
mod operations;
mod path;
pub mod file;
pub use filesystem_trait::{FileSystem, FileSystemFactory};
pub use inode::*;
pub use operations::*;
pub use path::*;

//0 is unknown, 1 is bad blocks, 2 is root
pub const ROOT_INODE_INDEX: u64 = 2;
pub static VFS: NoIntSpinlock<Vfs> = NoIntSpinlock::new(Vfs::new());

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[rustc_layout_scalar_valid_range_end(0xFFFF_FFFF_FFFF_FFFE)]
pub struct DeviceId(u64);

impl DeviceId {
    pub const fn new(id: u64) -> Self {
        unsafe { DeviceId(id) }
    }
}

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

///Always at least 1 element - root
pub type InodeIdentifierChain = Box<[InodeIdentifier]>;

pub struct Vfs {
    ///Map from disk guid to disk object (driver) and a list of partition guids
    disks: BTreeMap<Uuid, (Box<dyn BlockDevice + Send>, Vec<Uuid>)>,
    ///maps from filesystem type guid to filesystem driver factory
    filesystem_driver_factories: BTreeMap<Uuid, Box<dyn FileSystemFactory + Send>>,
    ///maps from partition guid to filesystem driver
    mounted_filesystems: BTreeMap<Uuid, Arc<dyn FileSystem + Send>>,
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
            mounted_filesystems: BTreeMap::new(),
            available_partitions: BTreeMap::new(),
            devices: BTreeMap::new(),
            device_counter: 1,
        }
    }

    pub fn allocate_device(&mut self) -> DeviceId {
        let id = self.device_counter;
        self.device_counter += 1;
        DeviceId::new(id)
    }
}

pub fn init() {
    let mut vfs = lock_w_info!(VFS);
    vfs.filesystem_driver_factories
        .insert(RfsFactory::UUID, Box::new(RfsFactory {}));
    vfs.filesystem_driver_factories
        .insert(DtmpfsFactory::UUID, Box::new(DtmpfsFactory {}));
}
