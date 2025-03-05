use core::alloc;
use std::{boxed::Box, collections::btree_map::BTreeMap};

use crate::drivers::disk::{Disk, FileSystem, FileSystemFactory};

use super::{btree::{BtreeNode, Key}, VIRTUAL_ONLY};

pub struct RfsFactory {}

impl RfsFactory {
    pub fn guid() -> u128 {
        0xb1b3b44dbece44dfba0e964a35a05a16
    }
}

impl FileSystemFactory for RfsFactory {
    fn guid(&self) -> u128 {
        Self::guid()
    }

    fn create(&self, disk: &'static mut dyn Disk) -> Box<dyn FileSystem> {
        Box::new(Rfs::new(disk))
    }
}

/*
 * Uses a B-tree for inodes. Will see number of children later
 * Uses a bitmap for free blocks every 0x8000 (32760) blocks. At fs creation, all bitmaps are 0,
 * except the last one, as it may extend past the disk
 * uses a bitmap for free inodes. Bitmap stops 4 bytes before the end of the block, as it contains
 * a pointer to the next bitmap block
 * */

///Inode 1 is root, 0 is unused. Inodes start at block 0
///Last 32bits of a block point to the next block if it exists, otherwise 0
///inode table is just a "file data" block, so also has a chain of blocks
///Block groups of size 256 blocks? 1MB
pub struct Rfs {
    ///bool is for modified
    ///Removing: Remove from cache, convert to VirtAddr, unmap
    inode_tree_cache: BTreeMap<u32, (bool, *mut BtreeNode)>,
    disk: &'static mut dyn Disk,
    root_block: u32,


    //virtual only
    block_counter: u32,
}

impl Rfs {
    pub fn new(disk: &'static mut dyn Disk) -> Self {
        Self {
            inode_tree_cache: BTreeMap::new(),
            disk,
            root_block: 1,
            block_counter: 1,
        }
    }

    pub fn allocate_block(&mut self) -> u32 {
        if VIRTUAL_ONLY {
            self.block_counter += 1;
            return self.block_counter; 
        }
        unimplemented!()
    }

    pub fn free_block(&mut self, _block: u32) {
        if VIRTUAL_ONLY {
            return; //nothing to do
        }
        unimplemented!()
    }

    //TODO: fix all modified bit
    pub fn get_node(&mut self, node_addr: u32) -> &mut (bool, *mut BtreeNode) {
        if let std::collections::btree_map::Entry::Vacant(e) = self.inode_tree_cache.entry(node_addr) {
            let data = BtreeNode::read_from_disk(self.disk, node_addr);
            e.insert((false, data));
        }

        self.inode_tree_cache.get_mut(&node_addr).unwrap()
    }

    pub fn add_node(&mut self, node_addr: u32, node: *mut BtreeNode) {
        self.inode_tree_cache.insert(node_addr, (true, node));
    }

    pub fn remove_node(&mut self, node_addr: u32) {
        self.inode_tree_cache.remove(&node_addr);
    }

    pub fn print_inode_tree(&mut self) {
        BtreeNode::print_inode_tree(self.root_block, 0, self);
    }

    pub fn add_key(&mut self, key: u32, value: u32) {
        let (_modified, node) = self.get_node(self.root_block);
        let node = &mut *node;
        let key = Key { index: key, indoe_block: value };
        let new_root = node.insert_key_root(self.root_block, key, self);
        if let Some(new_root) = new_root {
            self.root_block = new_root;
        }
    }
    
    pub fn remove_key(&mut self, key: u32) {
        let (_modified, node) = self.get_node(self.root_block);
        let node = &mut *node;
        let new_root = node.delete_key_root(self.root_block, key, self);
        if let Some(new_root) = new_root {
            self.root_block = new_root;
        }
    }
}

impl FileSystem for Rfs {}
