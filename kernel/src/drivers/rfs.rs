use bitfield::bitfield;

use super::disk::FileSystem;

const BLOCK_SIZE: u64 = 4096;

///Inode 1 is root, 0 is unused. Inodes start at block 0
///Last 32bits of a block point to the next block if it exists, otherwise 0
///inode table is just a "file data" block, so also has a chain of blocks
///Block groups of size 256 blocks? 1MB
pub struct Rfs {}

impl FileSystem for Rfs {
    fn guid(&self) -> u128 {
        0xb1b3b44dbece44dfba0e964a35a05a16
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DirEntry {
    pub inode: u32,
    pub name: [u8; 128], 
}

//gaol: inode is 128 bytes long
#[repr(C)]
#[derive(Debug)]
pub struct Inode {
    pub start_block: u32,
    pub block_length: u32,
}

//max size: block size
pub struct GroupHeader {
    bitmask_0: u128,
    bitmask_1: u128,
}

//unused for now, we don't need permissions
bitfield! {
    struct InodeFlags(u16);
    impl Debug;
    u_read, set_u_read: 0;
    u_write, set_u_write: 1;
    u_exec, set_u_exec: 2;
    g_read, set_g_read: 3;
    g_write, set_g_write: 4;
    g_exec, set_g_exec: 5;
    o_read, set_o_read: 6;
    o_write, set_o_write: 7;
    o_exec, set_o_exec: 8;
}
