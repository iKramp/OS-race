use uuid::Uuid;

use super::{
    DirEntry, Inode, InodeBitmask, InodeSize, SuperBlock,
    btree::{BtreeNode, Key},
};
use crate::{
    drivers::disk::{FileSystem, FileSystemFactory, MountedPartition},
    memory::{PAGE_TREE_ALLOCATOR, paging::LiminePat, physical_allocator::BUDDY_ALLOCATOR},
    vfs::{self, InodeType},
};
use core::{ffi, fmt::write};
use std::{
    PAGE_ALLOCATOR,
    boxed::Box,
    collections::btree_map::BTreeMap,
    mem_utils::{PhysAddr, VirtAddr, get_at_virtual_addr, memset_virtual_addr, set_at_physical_addr, set_at_virtual_addr},
    println,
    vec::{self, Vec},
};

const GROUP_BLOCK_SIZE: u64 = 4096 * 8;

pub struct RfsFactory {}

impl RfsFactory {
    pub fn guid() -> Uuid {
        Uuid::from_u128(0xb1b3b44dbece44dfba0e964a35a05a16)
    }
}

impl FileSystemFactory for RfsFactory {
    fn guid(&self) -> Uuid {
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
                .set_pat(LiminePat::UC);
        }
        for i in 0..self.groups {
            self.partition
                .read(i as usize * GROUP_BLOCK_SIZE as usize, 1, &[group_memory]);
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
                                .write(i as usize * GROUP_BLOCK_SIZE as usize, 1, &[group_memory]);
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
                .set_pat(LiminePat::UC);
        }
        let group = block / GROUP_BLOCK_SIZE as u32;
        let block_in_group = block % GROUP_BLOCK_SIZE as u32;
        let qword = block_in_group / 64;
        let bit = block_in_group % 64;

        self.partition
            .read(group as usize * GROUP_BLOCK_SIZE as usize, 1, &[group_memory]);
        let mut qword_data: u64 = unsafe { *get_at_virtual_addr(group_mem_binding + VirtAddr(qword as u64 * 8)) };
        assert!(qword_data & (1 << bit) != 0, "Block already free");
        qword_data &= !(1 << bit);
        unsafe {
            set_at_virtual_addr(group_mem_binding + VirtAddr(qword as u64 * 8), qword_data);
        }
    }

    pub fn allocate_inode(&mut self) -> u32 {
        let block_memory = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let block_mem_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(block_memory)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(block_mem_binding)
                .set_pat(LiminePat::UC);
        }
        self.partition.read(1, 1, &[block_memory]);
        let superblock: &mut SuperBlock = unsafe { get_at_virtual_addr(block_mem_binding) };
        let mut next_ptr = superblock.inode_bitmask;
        let mut block_index = 0;
        loop {
            self.partition.read(next_ptr as usize * 8, 8, &[block_memory]);
            let bitmask: &mut InodeBitmask = unsafe { get_at_virtual_addr(block_mem_binding) };
            for (bit_index, byte_mask) in bitmask.inodes.iter_mut().enumerate() {
                if *byte_mask != 0xFF {
                    for j in 0..8 {
                        if *byte_mask & (1 << j) == 0 {
                            *byte_mask |= 1 << j;
                            self.partition.write(next_ptr as usize * 8, 8, &[block_memory]);
                            unsafe { BUDDY_ALLOCATOR.deallocate_frame(block_memory) };
                            return block_index as u32 * 8 * bitmask.inodes.len() as u32 + (bit_index as u32 * 8) + j;
                        }
                    }
                }
            }
            block_index += 1;
            if bitmask.next_ptr == 0 {
                let new_block = self.allocate_block();
                bitmask.next_ptr = new_block;
                self.partition.write(next_ptr as usize * 8, 8, &[block_memory]);
                unsafe { std::mem_utils::memset_virtual_addr(block_mem_binding, 0, 4096) };
                self.partition.write(new_block as usize * 8, 1, &[block_memory]);
                bitmask.inodes[0] = 1;
                unsafe { PAGE_ALLOCATOR.deallocate(block_mem_binding) };
                return block_index as u32 * 8 * bitmask.inodes.len() as u32;
            } else {
                next_ptr = bitmask.next_ptr;
            }
        }
    }

    pub fn remove_inode_from_bitmask(&mut self, inode_index: u32) {
        let block_memory = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let block_mem_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(block_memory)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(block_mem_binding)
                .set_pat(LiminePat::UC);
        }
        self.partition.read(1, 1, &[block_memory]);
        let superblock: &mut SuperBlock = unsafe { get_at_virtual_addr(block_mem_binding) };
        let mut next_ptr = superblock.inode_bitmask;
        self.partition.read(next_ptr as usize * 8, 8, &[block_memory]);
        let mut inode_bitmask: &mut InodeBitmask = unsafe { get_at_virtual_addr(block_mem_binding) };

        for _i in 0..(inode_index / (inode_bitmask.inodes.len() as u32 * 8)) {
            self.partition.read(inode_bitmask.next_ptr as usize * 8, 8, &[block_memory]);
            inode_bitmask = unsafe { get_at_virtual_addr(block_mem_binding) };
            next_ptr = inode_bitmask.next_ptr;
        }
        let byte_index = (inode_index % (inode_bitmask.inodes.len() as u32 * 8)) / 8;
        let bit_index = (inode_index % (inode_bitmask.inodes.len() as u32 * 8)) % 8;
        inode_bitmask.inodes[byte_index as usize] &= !(1 << bit_index);
        self.partition.write(next_ptr as usize * 8, 8, &[block_memory]);
        unsafe { PAGE_ALLOCATOR.deallocate(block_mem_binding) };
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

    ///removes node from cache
    pub fn remove_inode_cache_entry(&mut self, node_addr: u32) {
        self.inode_tree_cache.remove(&node_addr);
    }

    pub fn clean_after_operation(&mut self) {
        for (block, (modified, node)) in self.inode_tree_cache.iter_mut() {
            if *modified {
                node.write_to_disk(&mut self.partition, *block);
                node.drop();
                *modified = false;
            }
        }
        self.inode_tree_cache.clear();
    }

    pub fn format_partition(&mut self) {
        let whole_blocks = self.partition.partition.size_sectors as u64 / 8;
        let whole_groups = whole_blocks / GROUP_BLOCK_SIZE;
        let last_group_blocks = whole_blocks % GROUP_BLOCK_SIZE;
        let group_memory = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let group_mem_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(group_memory)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(group_mem_binding)
                .set_pat(LiminePat::UC);
        }
        unsafe {
            memset_virtual_addr(group_mem_binding, 0, 4096);
            set_at_virtual_addr::<u8>(group_mem_binding, 1);
        }

        //----------Initialize free block tables----------
        for i in 0..whole_groups {
            self.partition
                .write(i as usize * GROUP_BLOCK_SIZE as usize, 8, &[group_memory]);
        }
        let last_group_invalid = GROUP_BLOCK_SIZE - last_group_blocks;
        let last_group_invalid_partial = if last_group_invalid % 8 == 0 {
            0
        } else {
            (0xFF >> (8 - last_group_invalid % 8)) << (8 - last_group_invalid % 8)
        };
        for i in 0..(last_group_invalid / 8) {
            unsafe {
                set_at_virtual_addr::<u8>(group_mem_binding + VirtAddr(4095 - i), 0xFF);
            }
        }
        unsafe {
            set_at_virtual_addr::<u8>(
                group_mem_binding + VirtAddr(4095 - last_group_invalid / 8),
                last_group_invalid_partial,
            );
        }
        self.partition
            .write(whole_groups as usize * GROUP_BLOCK_SIZE as usize, 1, &[group_memory]);
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, 4096) };

        //----------Initialize header at block 1----------
        let header = SuperBlock {
            inode_tree: 2,
            inode_bitmask: 0,
        };
        unsafe { set_at_virtual_addr(group_mem_binding, header) };
        self.partition.write(1, 1, &[group_memory]);
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, core::mem::size_of::<SuperBlock>()) };

        //----------Initialize root node at block 2, with a key for root----------
        let root_node = BtreeNode::new();
        root_node.set_key(
            0,
            Key {
                index: 2,
                inode_block: 3,
            },
        );
        unsafe { set_at_virtual_addr(group_mem_binding, root_node) };
        self.partition.write(2, 1, &[group_memory]);

        //i can clean like this because key is the first field
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, core::mem::size_of::<Key>()) };

        //----------Initialize root inode block at block 3----------
        let root_inode = Inode {
            size: InodeSize(core::mem::size_of::<[DirEntry; 2]>() as u64), //size 0, 0 levels of pointers
            inode_type_mode: InodeType::new_dir(0o755),
            link_count: 0,
            uid: 0,
            gid: 0,
            access_time: 0,
            modification_time: 0,
            stat_change_time: 0,
        };
        unsafe { set_at_virtual_addr(group_mem_binding, root_inode) };

        //write entries for itself and its parent, both actually being itself
        let mut root_dir_entry = DirEntry {
            inode: 3,
            name: [0; 128],
        };
        root_dir_entry.name[0] = b'.';
        unsafe {
            set_at_virtual_addr(
                group_mem_binding + VirtAddr(core::mem::size_of::<Inode>() as u64),
                root_dir_entry.clone(),
            )
        };
        root_dir_entry.name[1] = b'.';
        unsafe {
            set_at_virtual_addr(
                group_mem_binding + VirtAddr((core::mem::size_of::<Inode>() + core::mem::size_of::<DirEntry>()) as u64),
                root_dir_entry,
            )
        };

        unsafe {
            PAGE_ALLOCATOR.deallocate(group_mem_binding);
        }
    }

    ///this function will not write the inode frame. That is the responsibility of the calling
    ///function
    fn increase_file_size(&mut self, inode_frame: VirtAddr, inode_block: u32, size_new: u64) {
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_frame) };
        let mut levels_curr = inode_data.size.ptr_levels() as u32;
        let size_old = inode_data.size.size();

        let mut max_file_size = 512 * 7;
        let mut levels_new: u32 = 0;
        while max_file_size < size_new {
            max_file_size *= 1024;
            levels_new += 1;
        }

        assert!(levels_new <= 3, "File too big");
        while levels_new > levels_curr {
            levels_curr += 1;
            let working_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
            let working_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(working_block)) };
            unsafe {
                PAGE_TREE_ALLOCATOR
                    .get_page_table_entry_mut(working_block_binding)
                    .set_pat(LiminePat::UC);
            }

            self.partition.read(inode_block as usize * 8 + 1, 7, &[working_block]);

            let new_block_index = self.allocate_block();
            self.partition.write(new_block_index as usize * 8, 7, &[working_block]);

            unsafe { set_at_virtual_addr(working_block_binding, new_block_index) };
        }

        inode_data.size.set_ptr_levels(levels_new as u64);
        inode_data.size.set_size(size_new);

        let mut blocks_old = size_old.div_ceil(4096);
        let blocks_new = size_new.div_ceil(4096);

        if levels_new == 0 {
            //no allocation is necessary
            return;
        } else if levels_new == 1 {
            assert!(blocks_new <= 512 * 7 / 4, "Function did not increase levels enough");
            let pointers = unsafe { get_at_virtual_addr::<[u32; 512 / 4 * 7]>(inode_frame + VirtAddr(512)) };
            for i in blocks_old..blocks_new {
                let new_block = self.allocate_block();
                pointers[i as usize] = new_block;
            }
        } else {
            //level = 2 or 3
            let mut pointers = unsafe { get_at_virtual_addr::<[u32; 512 / 4 * 7]>(inode_frame + VirtAddr(512)) };
            let pointer_capacity = u64::pow(1024, levels_new - 1);
            for i in 0..(512 / 4 * 7) {
                if blocks_old >= blocks_new {
                    //all allocated
                    break;
                }
                if pointer_capacity * (i + 1) <= blocks_old {
                    //nothing to do as this pointer is already filled
                    continue;
                }

                let (lower_frame, lower_frame_binding);
                if pointer_capacity * i < blocks_old {
                    //lower is partially allocated
                    lower_frame = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
                    lower_frame_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(lower_frame)) };
                    unsafe {
                        PAGE_TREE_ALLOCATOR
                            .get_page_table_entry_mut(lower_frame_binding)
                            .set_pat(LiminePat::UC);
                    }
                    self.partition.read(pointers[i as usize] as usize * 8, 8, &[lower_frame]);
                } else {
                    //lower did not exist yet
                    let lower_block_index = self.allocate_block();
                    pointers[i as usize] = lower_block_index;
                    lower_frame = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
                    lower_frame_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(lower_frame)) };
                    unsafe {
                        PAGE_TREE_ALLOCATOR
                            .get_page_table_entry_mut(lower_frame_binding)
                            .set_pat(LiminePat::UC);
                    }
                }
                self.allocate_blocks_for_size_increase(
                    levels_new - 1,
                    i as u32,
                    lower_frame_binding,
                    blocks_new as u32,
                    blocks_old as u32,
                );
                self.partition.write(pointers[i as usize] as usize * 8, 8, &[lower_frame]);
                unsafe { PAGE_ALLOCATOR.deallocate(lower_frame_binding) };
                blocks_old = pointer_capacity * (i + 1);
            }
        }
    }

    ///Block index must point to a block of only pointers. Will loop through pointers, skip any
    ///unnecessary, step into the last one, and allocate new pointers
    ///this block must be memory mapped and set to uncacheable. As such, this function will also
    ///not write it to disk or deallocate it by itself
    ///index of the pointer to this block in the level of that pointer, globally, not just in
    ///that pointer set (eg. sizes go beyond 1023)
    fn allocate_blocks_for_size_increase(
        &mut self,
        level: u32,
        ptr_index: u32,
        block_page: VirtAddr,
        blocks_new: u32,
        mut blocks_old: u32,
    ) {
        let pointers = unsafe { get_at_virtual_addr::<[u32; 1024]>(block_page) };
        let pointer_capacity = u32::pow(1024, level - 1);
        let blocks_before_current = pointer_capacity * ptr_index;

        if level == 1 {
            for i in 0..1024 {
                //ptr capacity is 1
                if blocks_old >= blocks_new {
                    break;
                }
                if blocks_before_current + i < blocks_old {
                    continue;
                }
                let new_block = self.allocate_block();
                pointers[i as usize] = new_block;
                blocks_old += 1;
            }
        }

        for i in 0..1024 {
            if blocks_old >= blocks_new {
                break;
            }
            if blocks_before_current + i * pointer_capacity < blocks_old {
                continue;
            }

            let (lower_frame, lower_frame_binding);
            if blocks_before_current + pointer_capacity * i < blocks_old {
                //lower is partially allocated
                lower_frame = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
                lower_frame_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(lower_frame)) };
                unsafe {
                    PAGE_TREE_ALLOCATOR
                        .get_page_table_entry_mut(lower_frame_binding)
                        .set_pat(LiminePat::UC);
                }
                self.partition.read(pointers[i as usize] as usize * 8, 8, &[lower_frame]);
            } else {
                //lower did not exist yet
                let lower_block_index = self.allocate_block();
                pointers[i as usize] = lower_block_index;
                lower_frame = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
                lower_frame_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(lower_frame)) };
                unsafe {
                    PAGE_TREE_ALLOCATOR
                        .get_page_table_entry_mut(lower_frame_binding)
                        .set_pat(LiminePat::UC);
                }
            }
            self.allocate_blocks_for_size_increase(
                level - 1,
                i + (ptr_index * 1024),
                lower_frame_binding,
                blocks_new,
                blocks_old,
            );
            self.partition.write(pointers[i as usize] as usize * 8, 8, &[lower_frame]);
            unsafe { PAGE_ALLOCATOR.deallocate(lower_frame_binding) };
            blocks_old = pointer_capacity * (i + 1);
        }
    }

    fn delete_block(&mut self, level: u32, block_index: u32) {
        let working_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let working_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(working_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(working_block_binding)
                .set_pat(LiminePat::UC);
        }
        let pointers = unsafe { get_at_virtual_addr::<[u32; 1024]>(working_block_binding) };
        for i in 0..1024 {
            if level == 1 {
                self.free_block(pointers[i]);
            } else {
                self.delete_block(level - 1, pointers[i]);
            }
        }
    }
}

impl FileSystem for Rfs {
    fn unmount(&mut self) {
        self.clean_after_operation();
        self.inode_tree_cache.clear();
    }

    fn read(&mut self, inode: u32, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]) {
        assert!(buffer.len() == (offset_bytes + size_bytes).div_ceil(4096) as usize);
        assert!(offset_bytes % 4096 == 0);
        let aligned_size = size_bytes.div_ceil(4096) * 4096;
        let root = self.get_node(self.root_block).1;
        let inode_block_index = root.find_inode_block(inode, self).unwrap();
        let inode_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let inode_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(inode_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(inode_block_binding)
                .set_pat(LiminePat::UC);
        }
        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]);
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };
        assert!(size_bytes + offset_bytes <= inode_data.size.size());
        let mut levels = inode_data.size.ptr_levels();
        if levels == 0 {
            self.partition.read(inode_block_index as usize * 8 + 1, 7, buffer);
            unsafe { PAGE_ALLOCATOR.deallocate(inode_block_binding) };
            return;
        }
        //read first level pointers
        self.partition.read(inode_block_index as usize * 8 + 1, 7, &[inode_block]);

        let mut pointers: Vec<u32> = std::Vec::new();
        for i in 0..(512 * 7 / 8) {
            pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + VirtAddr(i * 4)) });
        }

        let mut first_ptr = 0;
        let mut ptr_span = 4096 + (levels - 1) * 1024;
        while levels > 0 {
            let first_relevant = (offset_bytes - first_ptr) / ptr_span;
            let last_relevant = (offset_bytes + aligned_size - 1 - first_ptr) / ptr_span;
            first_ptr += first_relevant * ptr_span;
            ptr_span /= 1024;

            let mut new_pointers =
                std::Vec::with_capacity((pointers.len() - (last_relevant - first_relevant + 1) as usize) * 1024);
            for i in first_relevant..=last_relevant {
                self.partition.read(pointers[i as usize] as usize * 8, 8, &[inode_block]);
                for i in 0..1024 {
                    new_pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + VirtAddr(i * 4)) });
                }
            }
            pointers = new_pointers;
            levels -= 1;
        }

        //At this point each pointer points to a 4k region
        let first_relevant = (offset_bytes - first_ptr) / 4096;
        let last_relevant = (offset_bytes + aligned_size - 1 - first_ptr) / 4096;
        for i in first_relevant..=last_relevant {
            let i = i as usize;
            self.partition
                .read(pointers[i] as usize * 8, 8, &buffer[(i * 8)..(i * 8 + 8)]);
        }
        unsafe { PAGE_ALLOCATOR.deallocate(inode_block_binding) };

        self.clean_after_operation();
    }

    fn write(&mut self, inode: u32, offset: u64, size: u64, buffer: &[PhysAddr]) {
        assert!(offset % 4096 == 0);
        assert!(size.div_ceil(4096) <= buffer.len() as u64);
        //get info about file currently
        let root = self.get_node(self.root_block).1;
        let inode_block_index = root.find_inode_block(inode, self).unwrap();
        let inode_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let inode_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(inode_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(inode_block_binding)
                .set_pat(LiminePat::UC);
        }
        self.partition.read(inode_block_index as usize * 8, 8, &[inode_block]);
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };

        let size_curr = inode_data.size.size();
        let size_new = u64::max(offset + size, size_curr);
        if size_new > size_curr {
            self.increase_file_size(inode_block_binding, inode_block_index, size_new);
        }

        let mut levels = inode_data.size.ptr_levels();

        //Root now contains 1 pointer, to possibly data, table of pointers or a single pointer
        //to... if increased by >1 level

        if levels == 0 {
            assert!(size <= 512 * 7);
            self.partition.write(inode_block_index as usize * 8 + 1, 7, buffer);
        }

        self.partition.write(inode_block_index as usize * 8, 8, &[inode_block]);

        let mut pointers: Vec<u32> = std::Vec::new();
        for i in 0..(512 * 7 / 8) {
            pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + VirtAddr(512 + i * 4)) });
        }

        let aligned_size = size.div_ceil(4096) * 4096;

        let mut first_ptr = 0;
        let mut ptr_span = 4096 + (levels - 1) * 1024;
        while levels > 0 {
            let first_relevant = (offset - first_ptr) / ptr_span;
            let last_relevant = (offset + aligned_size - 1 - first_ptr) / ptr_span;
            first_ptr += first_relevant * ptr_span;
            ptr_span /= 1024;

            let mut new_pointers =
                std::Vec::with_capacity((pointers.len() - (last_relevant - first_relevant + 1) as usize) * 1024);
            for i in first_relevant..=last_relevant {
                self.partition.read(pointers[i as usize] as usize * 8, 8, &[inode_block]);
                for i in 0..1024 {
                    new_pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + VirtAddr(i * 4)) });
                }
            }
            pointers = new_pointers;
            levels -= 1;
        }

        //with write to whole blocks we do not repsect size

        //At this point each pointer points to a 4k region
        let first_relevant = (offset - first_ptr) / 4096;
        let last_relevant = (offset + aligned_size - 1 - first_ptr) / 4096;
        for i in first_relevant..=last_relevant {
            let i = i as usize;
            self.partition
                .write(pointers[i] as usize * 8, 8, &buffer[(i * 8)..(i * 8 + 8)]);
        }
        unsafe { PAGE_ALLOCATOR.deallocate(inode_block_binding) };

        self.clean_after_operation();
    }

    fn stat(&mut self, inode: u32) -> crate::vfs::Inode {
        let root = self.get_node(self.root_block).1;
        let inode_block_index = root.find_inode_block(inode, self).unwrap();
        let inode_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let inode_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(inode_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(inode_block_binding)
                .set_pat(LiminePat::UC);
        }
        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]);
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };
        let vfs_inode = inode_data.to_vfs(inode, &self.partition.partition);
        unsafe { PAGE_ALLOCATOR.deallocate(inode_block_binding) };
        self.clean_after_operation();
        vfs_inode
    }

    fn set_stat(&mut self, inode_index: u32, vfs_inode_data: vfs::Inode) {
        let root = self.get_node(self.root_block).1;
        let inode_block_index = root.find_inode_block(inode_index, self).unwrap();
        let inode_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let inode_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(inode_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(inode_block_binding)
                .set_pat(LiminePat::UC);
        }
        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]);
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };
        inode_data.from_vfs_old(vfs_inode_data);
        self.partition.write(inode_block_index as usize * 8, 1, &[inode_block]);
        unsafe { PAGE_ALLOCATOR.deallocate(inode_block_binding) };
        self.clean_after_operation();
    }

    fn create(
        &mut self,
        name: std::string::String,
        parent_dir: u32,
        type_mode: crate::vfs::InodeType,
        uid: u16,
        gid: u16,
    ) -> crate::vfs::Inode {
        let new_inode_block_index = self.allocate_block();
        let inode_index = self.allocate_inode();
        let inode = Inode {
            size: InodeSize(0),
            inode_type_mode: type_mode,
            link_count: 1,
            uid,
            gid,
            access_time: 0,
            modification_time: 0,
            stat_change_time: 0,
        };
        let inode_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let inode_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(inode_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(inode_block_binding)
                .set_pat(LiminePat::UC);
        }
        let vfs_inode = inode.to_vfs(inode_index, &self.partition.partition);
        unsafe { set_at_virtual_addr(inode_block_binding, inode) };
        self.partition.write(new_inode_block_index as usize * 8, 1, &[inode_block]);

        let root = self.get_node(self.root_block).1;
        root.insert_key_root(
            self.root_block,
            Key {
                index: inode_index,
                inode_block: new_inode_block_index,
            },
            self,
        );

        self.link(inode_index, parent_dir, name);

        unsafe { PAGE_ALLOCATOR.deallocate(inode_block_binding) };

        vfs_inode
    }

    fn remove(&mut self, inode: u32) {
        let working_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let working_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(working_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(working_block_binding)
                .set_pat(LiminePat::UC);
        }
        let root = self.get_node(self.root_block).1;
        let inode_block_index = root.find_inode_block(inode, self).unwrap();
        self.partition.read(inode_block_index as usize * 8, 8, &[working_block]);
        let partition = unsafe { get_at_virtual_addr::<Inode>(working_block_binding) };
        let levels = partition.size.ptr_levels() as u32;

        if levels == 0 {
            self.free_block(inode_block_index);
        } else {
            let pointers = unsafe { get_at_virtual_addr::<[u32; 4026 / 8 * 7 / 4]>(working_block_binding + VirtAddr(512)) };
            for i in 0..(4096 / 8 * 7 / 4) {
                if levels == 1 {
                    self.free_block(pointers[i]);
                } else {
                    self.delete_block(levels - 1, pointers[i]);
                }
            }
        }
        unsafe { PAGE_ALLOCATOR.deallocate(working_block_binding) };
        self.remove_inode_cache_entry(inode);
        root.delete_key_root(self.root_block, inode, self);
        self.remove_inode_from_bitmask(inode);
        self.clean_after_operation();
    }

    fn link(&mut self, inode_index: u32, parent_inode: u32, name: std::string::String) {
        let root = self.get_node(self.root_block).1;
        let inode_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let inode_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(inode_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(inode_block_binding)
                .set_pat(LiminePat::UC);
        }

        let inode_block_index = root.find_inode_block(parent_inode, self).unwrap();
        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]);
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };
        let offset = inode_data.size.size();

        let needs_second_block = (offset + core::mem::size_of::<DirEntry>() as u64) % 4096 < (offset % 4096);
        let (second_block, second_block_binding);
        if needs_second_block {
            second_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
            second_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(second_block)) };
            unsafe {
                PAGE_TREE_ALLOCATOR
                    .get_page_table_entry_mut(second_block_binding)
                    .set_pat(LiminePat::UC);
            }
        } else {
            second_block = PhysAddr(0);
            second_block_binding = VirtAddr(0);
        }

        if offset % 4096 != 0 {
            self.read(parent_inode, offset & (!0xFFF), 4096, &[inode_block]);
        }
        let name_bytes = name.as_bytes();
        let mut name_byte_arr: [u8; 128] = [0; 128];
        for char in name_bytes.iter().enumerate() {
            name_byte_arr[char.0] = *char.1;
        }
        let temp_offset = offset & 0xFFF;
        let dir_entry = DirEntry {
            inode: inode_index,
            name: name_byte_arr,
        };
        let dir_entry_bytes = &dir_entry as *const DirEntry as *const u8;

        for i in 0..core::mem::size_of::<DirEntry>() {
            let new_off = temp_offset + i as u64;
            if new_off & (!0xFFF) == 0 {
                unsafe { set_at_virtual_addr(inode_block_binding + VirtAddr(new_off), dir_entry_bytes.add(i).read()) }
            } else {
                unsafe { set_at_virtual_addr(second_block_binding + VirtAddr(new_off % 4096), dir_entry_bytes.add(i).read()) }
            }
        }

        let write_size = if needs_second_block {
            8192
        } else {
            offset + core::mem::size_of::<DirEntry>() as u64
        };
        let buffers: &[PhysAddr] = if needs_second_block {
            &[inode_block, second_block]
        } else {
            &[inode_block]
        };
        self.write(parent_inode, offset & (!0xFFF), write_size, buffers);

        if needs_second_block {
            unsafe { PAGE_ALLOCATOR.deallocate(second_block_binding) };
        }
        unsafe { PAGE_ALLOCATOR.deallocate(inode_block_binding) };
        self.clean_after_operation();
    }

    fn truncate(&mut self, inode: u32, size: u64) {
        todo!()
    }

    fn rename(&mut self, inode: u32, parent_inode: u32, name: std::string::String) {
        let root_block_index = self.get_node(self.root_block).1;
        let parent_inode_block_index = root_block_index.find_inode_block(parent_inode, self).unwrap();
        let working_block = unsafe { BUDDY_ALLOCATOR.allocate_frame() };
        let working_block_binding = unsafe { PAGE_ALLOCATOR.allocate(Some(working_block)) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(working_block_binding)
                .set_pat(LiminePat::UC);
        }
        self.partition
            .read(parent_inode_block_index as usize * 8, 1, &[working_block]);
        let parent_inode_data = unsafe { get_at_virtual_addr::<Inode>(working_block_binding) };
        let dir_size = parent_inode_data.size.size();
        let dir_block_count = dir_size.div_ceil(4096);

        let mut frames = Vec::with_capacity(dir_block_count as usize);
        for _ in 0..dir_block_count {
            frames.push(unsafe { BUDDY_ALLOCATOR.allocate_frame() });
        }
        let folder_binding = unsafe { PAGE_ALLOCATOR.mmap_contigious(&frames) };
        for i in 0..dir_block_count {
            unsafe {
                PAGE_TREE_ALLOCATOR
                    .get_page_table_entry_mut(folder_binding + VirtAddr(i * 4096))
                    .set_pat(LiminePat::UC);
            }
        }

        self.read(parent_inode, 0, dir_size, &frames);
        let mut affected_inode = 0;
        for i in 0..(dir_size / core::mem::size_of::<DirEntry>() as u64) {
            let dir_entry = unsafe {
                get_at_virtual_addr::<DirEntry>(folder_binding + VirtAddr(i * core::mem::size_of::<DirEntry>() as u64))
            };
            if dir_entry.inode == inode {
                let name_bytes = name.as_bytes();
                let mut name_byte_arr: [u8; 128] = [0; 128];
                for char in name_bytes.iter().enumerate() {
                    name_byte_arr[char.0] = *char.1;
                }
                let mut new_dir_entry = dir_entry.clone();
                new_dir_entry.name = name_byte_arr;
                unsafe {
                    set_at_virtual_addr(
                        folder_binding + VirtAddr(i * core::mem::size_of::<DirEntry>() as u64),
                        new_dir_entry,
                    );
                }
                affected_inode = i;
                break;
            }
        }
        let affected_block = affected_inode * core::mem::size_of::<DirEntry>() as u64 / 4096;
        let next_block_affeted =
            ((affected_inode + 1) * core::mem::size_of::<DirEntry>() as u64 - 1) / 4096 == affected_block + 1;
        let write_size = if next_block_affeted { 8192 } else { 4096 };
        let buffers = if next_block_affeted {
            &frames[affected_block as usize..(affected_block + 2) as usize]
        } else {
            &frames[affected_block as usize..(affected_block + 1) as usize]
        };
        self.write(parent_inode, affected_block * 4096, write_size, buffers);

        for i in 0..dir_block_count {
            unsafe { PAGE_ALLOCATOR.deallocate(folder_binding + VirtAddr(i * 4096)) };
        }
        unsafe { PAGE_ALLOCATOR.deallocate(folder_binding) };
        unsafe { PAGE_ALLOCATOR.deallocate(working_block_binding) };

        self.clean_after_operation();
    }
}
