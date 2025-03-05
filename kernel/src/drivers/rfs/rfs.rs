use super::{btree::{BtreeNode, Key}, DirEntry, Inode, InodeSize, SuperBlock};
use crate::{
    drivers::disk::{FileSystem, FileSystemFactory, MountedPartition},
    memory::{paging::LiminePat, physical_allocator::BUDDY_ALLOCATOR, PAGE_TREE_ALLOCATOR}, vfs::InodeType,
};
use std::{
    boxed::Box, collections::btree_map::BTreeMap, mem_utils::{get_at_virtual_addr, set_at_physical_addr, set_at_virtual_addr, VirtAddr}, println, vec, PAGE_ALLOCATOR
};

const GROUP_BLOCK_SIZE: u64 = 4096 * 8;

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

    fn mount(&self, partition: MountedPartition) -> Box<dyn FileSystem> {
        Box::new(Rfs::new(partition))
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
    root_block: u32,
    groups: u32,
    blocks: u32,
    partition: MountedPartition,
}

impl Rfs {
    pub fn new(partition: MountedPartition) -> Self {
        let blocks = partition.partition.size_sectors as u32 / 8;
        let groups = blocks.div_ceil(GROUP_BLOCK_SIZE as u32);
        println!("it is initialized");
        Self {
            inode_tree_cache: BTreeMap::new(),
            root_block: 1,
            partition,
            groups,
            blocks,
        }
    }

    pub fn allocate_block(&mut self) -> u32 {
        let group_memory = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let group_mem_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(group_memory)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(group_mem_binding)
                .set_pat(LiminePat::WC);
        }
        for i in 0..self.groups {
            self.partition
                .read(i as usize * GROUP_BLOCK_SIZE as usize, 1, vec![group_memory]);
            for j in (0..4096).step_by(8) {
                let qword: u64 = unsafe { *get_at_virtual_addr(group_mem_binding + VirtAddr(j)) };
                if qword != 0xFFFFFFFFFFFFFFFF {
                    for k in 0..64 {
                        if qword & (1 << k) == 0 {
                            let allocated = qword | (1 << k);
                            unsafe {
                                set_at_virtual_addr(group_mem_binding + VirtAddr(j), allocated);
                            }
                            self.partition
                                .write(i as usize * GROUP_BLOCK_SIZE as usize, 1, vec![group_memory]);
                            unsafe { BUDDY_ALLOCATOR.deallocate_frame(group_memory) };
                            return i * GROUP_BLOCK_SIZE as u32 + j as u32 * 64 + k;
                        }
                    }
                }
            }
        }
        panic!("No free blocks")
    }

    pub fn free_block(&mut self, block: u32) {
        let group_memory = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let group_mem_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(group_memory)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(group_mem_binding)
                .set_pat(LiminePat::WC);
        }
        let group = block / GROUP_BLOCK_SIZE as u32;
        let block_in_group = block % GROUP_BLOCK_SIZE as u32;
        let qword = block_in_group / 64;
        let bit = block_in_group % 64;

        self.partition
            .read(group as usize * GROUP_BLOCK_SIZE as usize, 1, vec![group_memory]);
        let mut qword_data: u64 = unsafe { *get_at_virtual_addr(group_mem_binding + VirtAddr(qword as u64 * 8)) };
        assert!(qword_data & (1 << bit) != 0, "Block already free");
        qword_data &= !(1 << bit);
        unsafe {
            set_at_virtual_addr(group_mem_binding + VirtAddr(qword as u64 * 8), qword_data);
        }
    }

    pub fn get_node(&mut self, node_addr: u32) -> &mut (bool, *mut BtreeNode) {
        if let std::collections::btree_map::Entry::Vacant(e) = self.inode_tree_cache.entry(node_addr) {
            let data = BtreeNode::read_from_disk(&mut self.partition, node_addr);
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

    pub fn format_partition(&mut self) {
        let whole_blocks = self.partition.partition.size_sectors as u64 / 8;
        let whole_groups = whole_blocks / GROUP_BLOCK_SIZE;
        let last_group_blocks = whole_blocks % GROUP_BLOCK_SIZE;
        let group_memory = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let group_mem_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(group_memory)) };

        //----------Initialize free block tables----------
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(group_mem_binding)
                .set_pat(LiminePat::WC);
        }
        for i in 0..whole_groups {
            unsafe {
                set_at_physical_addr::<u8>(group_memory, 1);
            }
            self.partition
                .write(i as usize * GROUP_BLOCK_SIZE as usize, 1, vec![group_memory]);
        }
        let last_group_invalid = GROUP_BLOCK_SIZE - last_group_blocks;
        let last_group_invalid_partial = (0xFF >> (8 - last_group_invalid % 8)) << (8 - last_group_invalid % 8);
        for i in 0..(last_group_invalid / 8) {
            unsafe {
                set_at_virtual_addr::<u8>(group_mem_binding + VirtAddr(4095 - i as u64), 0xFF);
            }
        }
        unsafe {
            set_at_virtual_addr::<u8>(
                group_mem_binding + VirtAddr(4095 - last_group_invalid / 8 as u64),
                last_group_invalid_partial,
            );
        }
        self.partition
            .write(whole_groups as usize * GROUP_BLOCK_SIZE as usize, 1, vec![group_memory]);
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, 4096) };

        //----------Initialize header at block 1----------
        let header = SuperBlock { inode_tree: 2 };
        unsafe { set_at_virtual_addr(group_mem_binding, header) };
        self.partition.write(1, 1, vec![group_memory]);
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, core::mem::size_of::<SuperBlock>()) };

        //----------Initialize root node at block 2, with a key for root----------
        let root_node = BtreeNode::new();
        root_node.set_key(0, Key { index: 2, indoe_block: 3 });
        unsafe { set_at_virtual_addr(group_mem_binding, root_node) };
        self.partition.write(2, 1, vec![group_memory]);

        //i can clean like this because key is the first field
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, core::mem::size_of::<Key>()) }; 

        //----------Initialize root inode block at block 3----------
        let root_inode = Inode {
            size: InodeSize(core::mem::size_of::<[DirEntry; 2]>() as u64), //size 0, 0 levels of pointers
            inode_type_mode: InodeType::new_dir(0o755),
            link_count: 0,
            uid: 0,
            gid: 0,
        };
        unsafe { set_at_virtual_addr(group_mem_binding, root_inode) };

        //write entries for itself and its parent, both actually being itself
        let mut root_dir_entry = DirEntry {
            inode: 3,
            name: [0; 128],
        };
        root_dir_entry.name[0] = b'.';
        unsafe { set_at_virtual_addr(group_mem_binding + VirtAddr(core::mem::size_of::<Inode>() as u64), root_dir_entry.clone()) };
        root_dir_entry.name[1] = b'.';
        unsafe { set_at_virtual_addr(group_mem_binding + VirtAddr((core::mem::size_of::<Inode>() + core::mem::size_of::<DirEntry>()) as u64), root_dir_entry) };

        unsafe {
            PAGE_ALLOCATOR.deallocate(group_mem_binding);
        }
    }
}

impl FileSystem for Rfs {
    fn unmount(&self) {
        todo!()
    }

    fn read(&self, inode: u32, offset: u32, size: u32, buffer: std::Vec<std::mem_utils::PhysAddr>) {
        todo!()
    }

    fn write(&self, inode: u32, offset: u32, size: u32, buffer: std::Vec<std::mem_utils::PhysAddr>) {
        todo!()
    }

    fn stat(&self, inode: u32) -> crate::vfs::Inode {
        todo!()
    }

    fn create(&self, path: std::string::String, type_mode: crate::vfs::InodeType) -> crate::vfs::Inode {
        todo!()
    }

    fn remove(&self, inode: u32) {
        todo!()
    }

    fn link(&self, inode: u32, path: std::string::String) {
        todo!()
    }
}
