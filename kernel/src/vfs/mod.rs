use bitfield::bitfield;
use std::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    format,
    sync::mutex::Mutex,
    vec::Vec,
};
use uuid::Uuid;

use crate::drivers::{
    disk::{Disk, FileSystem, FileSystemFactory, Partition},
    rfs::RfsFactory,
};

mod fs_tree;
mod operations;
pub use operations::*;

//0 is unknown, 1 is bad blocks, 2 is root
pub const ROOT_INODE_INDEX: u32 = 2;
pub static VFS: Mutex<Vfs> = Mutex::new(Vfs::new());

///A wrapper type for path, that have been resolved to a list of path components
///That is, the path starts from root and does not contain any "." or ".." components
#[repr(transparent)]
pub struct ResolvedPath(Box<[Box<str>]>);

//just a wrapper
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

//this is returned by the stat() syscall
#[derive(Debug, Clone)]
pub struct Inode {
    pub index: u32,
    pub device: DeviceId, //some map to major/minor (minor are partitions)
    pub type_mode: InodeType,
    pub link_cnt: u16,
    pub uid: u16,
    pub gid: u16,
    ///this is set to a device uuid if the inode represents a device
    pub device_represented: Option<Uuid>,
    ///len of a symlink is the length of the pathname
    pub size: u64,
    //available if this represents a device, otherwise inherits from device
    pub preferred_block_size: u16,
    ///number of blocks used by this inode, in 512 byte units!!!!!
    pub blocks: u32,
    pub access_time: u32,
    pub modification_time: u32,
    pub stat_change_time: u32,
}

const FILE_MODE_MASK: u32 = 0xFFF00000;
const FILE_TYPE_MASK: u32 = 0xF000;
const PERM_MASK: u32 = 0x1FF;
const TEST: u32 = 0o4000;
//use this: https://man7.org/linux/man-pages/man7/inode.7.html
//internal fs inode types may differ (as there is no need for socket, block device,...) but rfs
//uses this
#[derive(Debug, Clone)]
pub struct InodeType(u32);

impl InodeType {
    pub fn get_flags(&self) -> InodeFlags {
        InodeFlags(self.0 & PERM_MASK)
    }

    pub fn is_socket(&self) -> bool {
        self.0 & FILE_TYPE_MASK == 0o140000
    }

    pub fn is_symlink(&self) -> bool {
        self.0 & FILE_TYPE_MASK == 0o120000
    }

    pub fn is_file(&self) -> bool {
        self.0 & FILE_TYPE_MASK == 0o100000
    }

    pub fn is_dir(&self) -> bool {
        self.0 & FILE_TYPE_MASK == 0o40000
    }

    pub fn is_block_device(&self) -> bool {
        self.0 & FILE_TYPE_MASK == 0o60000
    }

    pub fn is_char_device(&self) -> bool {
        self.0 & FILE_TYPE_MASK == 0o20000
    }

    pub fn is_fifo(&self) -> bool {
        self.0 & FILE_TYPE_MASK == 0o10000
    }

    pub fn new_dir(perms: u32) -> Self {
        InodeType(0o40000 | perms)
    }
}

//unused for now, we don't need permissions
bitfield! {
    pub struct InodeFlags(u32);
    impl Debug;
    pub suid, set_suid: 0x800;
    pub sgid, set_sgid: 0x400;
    pub sticky, set_sticky: 0x200;

    pub r_usr, set_r_usr: 0x100;
    pub w_usr, set_w_usr: 0x80;
    pub x_usr, set_x_usr: 0x40;

    pub r_grp, set_r_grp: 0x20;
    pub w_grp, set_w_grp: 0x10;
    pub x_grp, set_x_grp: 0x8;

    pub r_othr, set_r_othr: 0x4;
    pub w_othr, set_w_othr: 0x2;
    pub x_othr, set_x_othr: 0x1;
}

pub fn resolve_path(path: &str, working_dir: &str) -> ResolvedPath {
    if path.starts_with('/') {
        return ResolvedPath(resolve_single_path(path).into());
    } else {
        let resolved_path = resolve_single_path(format!("{}/{}", working_dir, path).as_str());
        return ResolvedPath(resolved_path);
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

    return path.into();
}
