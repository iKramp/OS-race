use bitfield::bitfield;
use std::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use crate::drivers::{
    disk::{Disk, FileSystem, FileSystemFactory, PartitionSchemeDriver},
    gpt::GPTDriver,
    rfs::RfsFactory,
};

///Map from disk guid to disk object (driver) and a list of partition guids
static mut DISKS: BTreeMap<u128, (Box<dyn Disk>, Vec<u128>)> = BTreeMap::new();
///maps from filesystem type guid to filesystem driver factory
static mut FILESYSTEM_DRIVER_FACTORIES: BTreeMap<u128, Box<dyn FileSystemFactory>> = BTreeMap::new();
///maps from partition guid to filesystem driver. Might be unused if we don't do caching
static mut MOUNTED_PARTITIONS: BTreeMap<u128, Box<dyn FileSystem>> = BTreeMap::new();

pub fn init() {
    unsafe {
        FILESYSTEM_DRIVER_FACTORIES.insert(RfsFactory::guid(), Box::new(RfsFactory {}));
    }
}

pub fn add_disk(mut disk: Box<dyn Disk>) {
    //for now only GPT
    let gpt_driver = GPTDriver {};
    let guid = gpt_driver.guid(&mut *disk);
    let partitions = gpt_driver.partitions(&mut *disk);
    let partition_guids: Vec<u128> = partitions.iter().map(|(guid, _)| *guid).collect();

    unsafe {
        DISKS.insert(guid, (disk, partition_guids));
    }
}

//this is returned by the stat() syscall
pub struct Inode {
    index: u32,
    device: u128, //some map to major/minor (minor are partitions)
    type_mode: InodeType,
    link_cnt: u16,
    uid: u16,
    gid: u16,
    ///this is set to a device uuid if the inode represents a device
    device_represented: u128,
    ///len of a symlink is the length of the pathname
    size: u64,
    //available if this represents a device, otherwise inherits from device
    preferred_block_size: u16,
    ///number of blocks used by this inode, in 512 byte units!!!!!
    blocks: u32,
    access_time: u32,
    modification_time: u32,
    stat_change_time: u32,
}

const FILE_MODE_MASK: u32 = 0xFFF00000;
const FILE_TYPE_MASK: u32 = 0xF000;
const PERM_MASK: u32 = 0x1FF;
const TEST: u32 = 0o4000;
//use this: https://man7.org/linux/man-pages/man7/inode.7.html
//internal fs inode types may differ (as there is no need for socket, block device,...) but rfs
//uses this
#[derive(Debug)]
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
