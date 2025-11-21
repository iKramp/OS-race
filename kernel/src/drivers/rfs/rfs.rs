use uuid::Uuid;

use super::{
    DirEntry, Inode, InodeBitmask, InodeSize, SuperBlock,
    btree::{BtreeNode, Key},
};
use crate::{
    drivers::{disk::MountedPartition, rfs::BLOCK_SIZE_SECTORS},
    memory::{PAGE_TREE_ALLOCATOR, paging::LiminePat, physical_allocator},
    vfs::{self, FileSystem, FileSystemFactory, InodeIndex, InodeType, ROOT_INODE_INDEX},
};
use core::str;
use std::{
    sync::arc::Arc, boxed::Box, collections::btree_map::BTreeMap, lock_w_info, mem_utils::{get_at_virtual_addr, memset_virtual_addr, set_at_virtual_addr, PhysAddr, VirtAddr}, sync::{async_lock::AsyncSpinlock, async_rw_lock::AsyncRWlock, lock_info::LockLocationInfo, no_int_spinlock::NoIntSpinlock}, vec::Vec
};

const GROUP_BLOCK_SIZE: u64 = 4096 * 8;

pub struct RfsFactory;

impl RfsFactory {
    pub const UUID: Uuid = Uuid::from_u128(0xb1b3b44dbece44dfba0e964a35a05a16);
}

#[async_trait::async_trait]
impl FileSystemFactory for RfsFactory {
    async fn mount(&self, partition: MountedPartition) -> Arc<dyn FileSystem + Send> {
        Arc::new(Rfs::new(partition).await)
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
#[derive(Debug)]
pub struct Rfs {
    ///bool is for modified
    ///Removing: Remove from cache, convert to VirtAddr, unmap
    inode_lock: AsyncSpinlock<()>,
    inode_tree_cache: BTreeMap<u32, (bool, VirtAddr)>,
    root_block: u32,

    //bye bye performance
    file_locks: NoIntSpinlock<BTreeMap<u32, Arc<AsyncRWlock<()>>>>,

    block_alloc_lock: AsyncSpinlock<()>,
    //written once
    groups: u32,
    //written once
    blocks: u32,
    //is Send + Sync
    partition: MountedPartition,
}

//safe because all fields are either Send + Sync, single write, or in a lock
unsafe impl Send for Rfs {}
unsafe impl Sync for Rfs {}

fn get_working_block() -> (PhysAddr, VirtAddr) {
    let working_block = physical_allocator::allocate_frame();
    let working_block_binding = unsafe { PAGE_TREE_ALLOCATOR.allocate(Some(working_block), false) };
    unsafe {
        PAGE_TREE_ALLOCATOR
            .get_page_table_entry_mut(working_block_binding)
            .unwrap()
            .set_pat(LiminePat::UC);
    }
    (working_block, working_block_binding)
}

impl Rfs {
    pub async fn new(partition: MountedPartition) -> Self {
        let blocks = partition.partition.size_sectors as u32 / 8;
        let groups = blocks.div_ceil(GROUP_BLOCK_SIZE as u32);
        let (working_block, working_block_binding) = get_working_block();

        partition.read(BLOCK_SIZE_SECTORS, 1, &[working_block]).await;
        let header = unsafe { get_at_virtual_addr::<SuperBlock>(working_block_binding) };
        let root_block = header.inode_tree;
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(working_block_binding) };

        // driver.format_partition();

        Self {
            inode_lock: AsyncSpinlock::new(()),
            inode_tree_cache: BTreeMap::new(),
            root_block,
            partition,
            groups,
            blocks,
            file_locks: NoIntSpinlock::new(BTreeMap::new()),
            block_alloc_lock: AsyncSpinlock::new(()),
        }
    }

    //can be used for btree operations when holding the inode lock
    unsafe fn to_mut_ptr(&self) -> *mut Self {
        self as *const Self as *mut Self
    }

    fn get_file_lock(&self, inode_index: u32) -> Arc<AsyncRWlock<()>> {
        let mut inode_locks = lock_w_info!(self.file_locks);
        let file_lock = inode_locks
            .entry(inode_index)
            .or_insert_with(|| Arc::new(AsyncRWlock::new(())))
            .clone();
        drop(inode_locks);
        file_lock
    }

    pub async fn allocate_block(&self) -> u32 {
        let lock = self.block_alloc_lock.lock();
        let (group_memory, group_mem_binding) = get_working_block();
        for i in 0..self.groups {
            self.partition.read(
                i as usize * GROUP_BLOCK_SIZE as usize * BLOCK_SIZE_SECTORS,
                1,
                &[group_memory],
            ).await;
            for j in (0..4096).step_by(8) {
                let qword: u64 = unsafe { *get_at_virtual_addr(group_mem_binding + j) };
                if qword != 0xFFFFFFFFFFFFFFFF {
                    for k in 0..64 {
                        if qword & (1 << k) == 0 {
                            let allocated = qword | (1 << k);
                            unsafe {
                                set_at_virtual_addr(group_mem_binding + j, allocated);
                            }
                            self.partition
                                .write(i as usize * GROUP_BLOCK_SIZE as usize, 1, &[group_memory])
                                .await;
                            drop(lock);

                            unsafe { physical_allocator::deallocate_frame(group_memory) };
                            return i * GROUP_BLOCK_SIZE as u32 + j as u32 * 64 + k;
                        }
                    }
                }
            }
        }
        drop(lock);
        panic!("No free blocks")
    }

    pub async fn free_block(&self, block: u32) {
        let lock = self.block_alloc_lock.lock();
        let (group_memory, group_mem_binding) = get_working_block();
        let group = block / GROUP_BLOCK_SIZE as u32;
        let block_in_group = block % GROUP_BLOCK_SIZE as u32;
        let qword = block_in_group / 64;
        let bit = block_in_group % 64;

        self.partition
            .read(group as usize * GROUP_BLOCK_SIZE as usize, 1, &[group_memory])
            .await;
        let mut qword_data: u64 = unsafe { *get_at_virtual_addr(group_mem_binding + qword as u64 * 8) };
        assert!(qword_data & (1 << bit) != 0, "Block already free");
        qword_data &= !(1 << bit);
        unsafe {
            set_at_virtual_addr(group_mem_binding + (qword as u64 * 8), qword_data);
        }
        drop(lock)
    }

    /// Safety
    /// must hold inode tree lock
    pub async unsafe fn allocate_inode(&self) -> u32 {
        let (block_memory, block_mem_binding) = get_working_block();
        self.partition.read(BLOCK_SIZE_SECTORS, 1, &[block_memory]).await;
        let superblock: &mut SuperBlock = unsafe { get_at_virtual_addr(block_mem_binding) };
        let mut next_ptr = superblock.inode_bitmask;
        let mut block_index = 0;
        loop {
            self.partition
                .read(next_ptr as usize * BLOCK_SIZE_SECTORS, 8, &[block_memory])
                .await;
            let bitmask: &mut InodeBitmask = unsafe { get_at_virtual_addr(block_mem_binding) };
            for (bit_index, byte_mask) in bitmask.inodes.iter_mut().enumerate() {
                if *byte_mask != 0xFF {
                    for j in 0..8 {
                        if *byte_mask & (1 << j) == 0 {
                            *byte_mask |= 1 << j;
                            self.partition.write(next_ptr as usize * 8, 8, &[block_memory]).await;
                            unsafe { physical_allocator::deallocate_frame(block_memory) };
                            return block_index as u32 * 8 * bitmask.inodes.len() as u32 + (bit_index as u32 * 8) + j;
                        }
                    }
                }
            }
            block_index += 1;
            if bitmask.next_ptr == 0 {
                let new_block = self.allocate_block().await;
                bitmask.next_ptr = new_block;
                self.partition.write(next_ptr as usize * 8, 8, &[block_memory]).await;
                unsafe { std::mem_utils::memset_virtual_addr(block_mem_binding, 0, 4096) };
                self.partition.write(new_block as usize * 8, 1, &[block_memory]).await;
                bitmask.inodes[0] = 1;
                unsafe { PAGE_TREE_ALLOCATOR.deallocate(block_mem_binding) };
                return block_index as u32 * 8 * bitmask.inodes.len() as u32;
            } else {
                next_ptr = bitmask.next_ptr;
            }
        }
    }

    pub async fn remove_inode_from_bitmask(&mut self, inode_index: u32) {
        let (block_memory, block_mem_binding) = get_working_block();
        self.partition.read(1, 1, &[block_memory]).await;
        let superblock: &mut SuperBlock = unsafe { get_at_virtual_addr(block_mem_binding) };
        let mut next_ptr = superblock.inode_bitmask;
        self.partition.read(next_ptr as usize * 8, 8, &[block_memory]).await;
        let mut inode_bitmask: &mut InodeBitmask = unsafe { get_at_virtual_addr(block_mem_binding) };

        let inode_lock = self.inode_lock.lock().await;
        for _i in 0..(inode_index / (inode_bitmask.inodes.len() as u32 * 8)) {
            self.partition
                .read(inode_bitmask.next_ptr as usize * 8, 8, &[block_memory])
                .await;
            inode_bitmask = unsafe { get_at_virtual_addr(block_mem_binding) };
            next_ptr = inode_bitmask.next_ptr;
        }
        let byte_index = (inode_index % (inode_bitmask.inodes.len() as u32 * 8)) / 8;
        let bit_index = (inode_index % (inode_bitmask.inodes.len() as u32 * 8)) % 8;
        inode_bitmask.inodes[byte_index as usize] &= !(1 << bit_index);
        self.partition.write(next_ptr as usize * 8, 8, &[block_memory]).await;
        drop(inode_lock);
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(block_mem_binding) };
    }

    /// Safety
    /// must hold inode tree lock
    pub async unsafe fn get_node(&self, node_block: u32) -> &mut (bool, VirtAddr) {
        let self_mut = unsafe { &mut *self.to_mut_ptr() };
        if let std::collections::btree_map::Entry::Vacant(e) = self_mut.inode_tree_cache.entry(node_block) {
            let data = BtreeNode::read_from_disk(&self.partition, node_block).await;
            e.insert((false, data));
        }

        self_mut.inode_tree_cache.get_mut(&node_block).unwrap()
    }

    /// Safety
    /// must hold inode tree lock
    pub unsafe fn add_node(&self, node_block: u32, node: VirtAddr) {
        let self_mut = unsafe { &mut *self.to_mut_ptr() };
        self_mut.inode_tree_cache.insert(node_block, (true, node));
    }

    /// Safety
    /// removes node from cache
    /// must hold inode tree lock
    pub unsafe fn remove_inode_cache_entry(&self, node_block: u32) {
        let self_mut = unsafe { &mut *self.to_mut_ptr() };
        self_mut.inode_tree_cache.remove(&node_block);
    }

    pub async fn clean_inode_tree_cache(&self) {
        let inode_lock = self.inode_lock.lock().await;
        let self_mut = unsafe { &mut *self.to_mut_ptr() };

        for (block, (modified, node)) in self_mut.inode_tree_cache.iter_mut() {
            let on_disk_ptr = BtreeNode::read_from_disk(&self.partition, *block).await;
            let on_disk = unsafe { &*(on_disk_ptr.0 as *const [u8; 4096]) };
            let in_mem = unsafe { &*(node.0 as *const [u8; 4096]) };
            if on_disk != in_mem && !*modified {
                panic!("Node was modified but not marked as such");
            }
            BtreeNode::drop(on_disk_ptr);

            if *modified {
                BtreeNode::write_to_disk(*node, &self.partition, *block).await;
                BtreeNode::drop(*node);
                *modified = false;
            }
        }
        self_mut.inode_tree_cache.clear();
        drop(inode_lock);
    }

    pub async fn format_partition(&self) {
        let whole_blocks = self.partition.partition.size_sectors as u64 / 8;
        assert!(whole_blocks >= 4, "Partition too small");
        let whole_groups = whole_blocks / GROUP_BLOCK_SIZE;
        let last_group_blocks = whole_blocks % GROUP_BLOCK_SIZE;
        let (group_memory, group_mem_binding) = get_working_block();
        unsafe {
            memset_virtual_addr(group_mem_binding, 0, 4096);

            //first 5 groups are taken
            set_at_virtual_addr::<u8>(group_mem_binding, 0b11111);
        }

        //----------Initialize free block tables----------
        for i in 0..whole_groups {
            self.partition
                .write(i as usize * GROUP_BLOCK_SIZE as usize, 8, &[group_memory])
                .await;
        }
        let last_group_invalid = GROUP_BLOCK_SIZE - last_group_blocks;
        let last_group_invalid_partial = last_group_invalid
            .unbounded_shr(8 - last_group_invalid as u32 % 8)
            .unbounded_shl(8 - last_group_invalid as u32 % 8);
        for i in 0..(last_group_invalid / 8) {
            unsafe {
                set_at_virtual_addr::<u8>(group_mem_binding + 4095 - i, 0xFF);
            }
        }
        unsafe {
            set_at_virtual_addr::<u8>(
                group_mem_binding + 4095 - (last_group_invalid / 8),
                last_group_invalid_partial as u8,
            );
        }
        self.partition
            .write(whole_groups as usize * GROUP_BLOCK_SIZE as usize, 8, &[group_memory])
            .await;
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, 4096) };

        //----------Initialize header at block 1----------
        let header = SuperBlock {
            inode_tree: 2,
            inode_bitmask: 4,
        };
        unsafe { set_at_virtual_addr(group_mem_binding, header) };
        self.partition.write(BLOCK_SIZE_SECTORS, 1, &[group_memory]).await;
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, core::mem::size_of::<SuperBlock>()) };

        //----------Initialize root node at block 2, with a key for root----------
        BtreeNode::set_key(
            group_mem_binding,
            0,
            Key {
                index: ROOT_INODE_INDEX as u32,
                inode_block: 3,
            },
        );

        self.partition.write(2 * BLOCK_SIZE_SECTORS, 1, &[group_memory]).await;

        //i can clean like this because key is the first field
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, core::mem::size_of::<Key>()) };

        //----------Initialize root inode block at block 3----------
        let root_inode = Inode {
            size: InodeSize(0), //size 0, 0 levels of pointers
            inode_type_mode: InodeType::new_dir(0o755),
            link_count: 0,
            uid: 0,
            gid: 0,
            access_time: 0,
            modification_time: 0,
            stat_change_time: 0,
        };
        unsafe { set_at_virtual_addr(group_mem_binding, root_inode) };

        self.partition.write(3 * BLOCK_SIZE_SECTORS, 1, &[group_memory]).await;
        unsafe { std::mem_utils::memset_virtual_addr(group_mem_binding, 0, 4096) };

        //---------------Initialize inode bitmask at block 4---------------
        for i in 1..BLOCK_SIZE_SECTORS as u32 {
            self.partition
                .write(4 * BLOCK_SIZE_SECTORS + i as usize, 1, &[group_memory])
                .await;
        }
        //indexes 0, 1, and 2 are used
        unsafe { set_at_virtual_addr::<u8>(group_mem_binding, 0b111) };
        self.partition.write(4 * BLOCK_SIZE_SECTORS, 1, &[group_memory]).await;

        unsafe {
            PAGE_TREE_ALLOCATOR.deallocate(group_mem_binding);
        }
    }

    #[allow(unreachable_code)]
    /// Safety
    /// file lock must be held
    async fn increase_file_size(&self, inode_frame_binding: VirtAddr, inode_frame: PhysAddr, inode_block: u32, size_new: u64) {
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_frame_binding) };
        let mut levels_curr = inode_data.size.ptr_levels() as u32;
        let size_old = inode_data.size.size();

        let mut max_file_size = 512 * 7;
        let mut levels_new: u32 = 0;
        while max_file_size < size_new {
            max_file_size *= 1024;
            levels_new += 1;
        }
        assert!(levels_new <= 3, "File too big");

        let (working_block, working_block_binding) = get_working_block();
        //increase file depth
        {
            while levels_new > levels_curr {
                levels_curr += 1;

                self.partition.read(inode_block as usize * 8 + 1, 7, &[working_block]).await;

                let new_block_index = self.allocate_block().await;
                self.partition.write(new_block_index as usize * 8, 7, &[working_block]).await;

                unsafe {
                    std::mem_utils::memset_virtual_addr(working_block_binding, 0, 512 * 7);
                    set_at_virtual_addr(working_block_binding, new_block_index)
                };
                self.partition.write(inode_block as usize * 8 + 1, 1, &[working_block]).await;
            }
        }

        inode_data.size.set_ptr_levels(levels_new as u64);
        inode_data.size.set_size(size_new);
        self.partition
            .write(inode_block as usize * BLOCK_SIZE_SECTORS, 1, &[inode_frame])
            .await;

        let blocks_old = size_old.div_ceil(4096);
        let blocks_new = size_new.div_ceil(4096);

        if levels_new == 0 {
            //no allocation is necessary
            unsafe { PAGE_TREE_ALLOCATOR.deallocate(working_block_binding) };
            return;
        }
        if levels_new == 1 {
            assert!(blocks_new <= 512 * 7 / 4, "Function did not increase levels enough");
            self.partition
                .read(inode_block as usize * BLOCK_SIZE_SECTORS + 1, 7, &[working_block])
                .await;
            let pointers = unsafe { get_at_virtual_addr::<[u32; 512 / 4 * 7]>(working_block_binding) };
            for i in blocks_old..blocks_new {
                let new_block = self.allocate_block().await;
                pointers[i as usize] = new_block;
            }
            self.partition
                .write(inode_block as usize * BLOCK_SIZE_SECTORS + 1, 7, &[working_block])
                .await;
        } else {
            //level = 2 or 3
            todo!("This probably doesn't work");
            let pointers = unsafe { get_at_virtual_addr::<[u32; 512 / 4 * 7]>(inode_frame_binding + 512) };
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

                let (lower_frame, lower_frame_binding) = get_working_block();

                if pointer_capacity * i < blocks_old {
                    //lower is partially allocated
                    self.partition
                        .read(pointers[i as usize] as usize * 8, 8, &[lower_frame])
                        .await;
                } else {
                    //lower did not exist yet
                    let lower_block_index = self.allocate_block().await;
                    pointers[i as usize] = lower_block_index;
                }
                self.allocate_blocks_for_size_increase(
                    levels_new - 1,
                    i as u32,
                    lower_frame_binding,
                    blocks_new as u32,
                    blocks_old as u32,
                )
                .await;
                self.partition
                    .write(pointers[i as usize] as usize * 8, 8, &[lower_frame])
                    .await;
                unsafe { PAGE_TREE_ALLOCATOR.deallocate(lower_frame_binding) };
                blocks_old = pointer_capacity * (i + 1);
            }
        }
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(working_block_binding) };
    }

    ///Block index must point to a block of only pointers. Will loop through pointers, skip any
    ///unnecessary, step into the last one, and allocate new pointers
    ///this block must be memory mapped and set to uncacheable. As such, this function will also
    ///not write it to disk or deallocate it by itself
    ///index of the pointer to this block in the level of that pointer, globally, not just in
    ///that pointer set (eg. sizes go beyond 1023)
    ///Pointer capacity is in blocks, not bytes
    ///Inode lock must be held
    async fn allocate_blocks_for_size_increase(
        &self,
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
            if blocks_old >= blocks_new {
                return;
            }
            for i in 0..1024 {
                //ptr capacity is 1
                if ptr_index + i < blocks_old {
                    continue;
                }
                let new_block = self.allocate_block().await;
                pointers[i as usize] = new_block;
            }
            return;
        }

        for i in 0..1024 {
            if blocks_old >= blocks_new {
                return;
            }
            if blocks_before_current + (i + 1) * pointer_capacity <= blocks_old {
                continue;
            }

            let (lower_frame, lower_frame_binding) = get_working_block();
            if blocks_before_current + pointer_capacity * i < blocks_old {
                //lower is partially allocated
                self.partition
                    .read(pointers[i as usize] as usize * 8, 8, &[lower_frame])
                    .await;
            } else {
                //lower did not exist yet
                let lower_block_index = self.allocate_block().await;
                pointers[i as usize] = lower_block_index;
            }
            Box::pin(self.allocate_blocks_for_size_increase(
                level - 1,
                i + (ptr_index * 1024),
                lower_frame_binding,
                blocks_new,
                blocks_old,
            )).await;
            self.partition
                .write(pointers[i as usize] as usize * 8, 8, &[lower_frame])
                .await;
            unsafe { PAGE_TREE_ALLOCATOR.deallocate(lower_frame_binding) };
            blocks_old = blocks_before_current + pointer_capacity * (i + 1);
        }
    }

    ///must hold inode lock
    async unsafe fn delete_block(&mut self, level: u32, block_index: u32) {
        let (working_block, working_block_binding) = get_working_block();
        self.partition.read(block_index as usize * 8, 8, &[working_block]).await;
        let pointers = unsafe { get_at_virtual_addr::<[u32; 1024]>(working_block_binding) };
        for i in 0..1024 {
            if level == 1 {
                self.free_block(pointers[i]).await;
            } else {
                unsafe { Box::pin(self.delete_block(level - 1, pointers[i])).await };
            }
        }
    }

    async unsafe fn read_locked(&self, inode: InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]) {
        if size_bytes == 0 {
            return;
        }
        assert!(buffer.len() == (offset_bytes + size_bytes).div_ceil(4096) as usize);
        assert!(offset_bytes % 4096 == 0);
        let aligned_size = size_bytes.div_ceil(4096) * 4096;
    
        let inode_tree_lock = self.inode_lock.lock().await;

        let root = unsafe { self.get_node(self.root_block).await.1 };
        let inode_block_index = BtreeNode::find_inode_block(root, inode as u32, self).await.unwrap();
        drop(inode_tree_lock); //file inode lock is held, so file won't move. Found the block
        let (inode_block, inode_block_binding) = get_working_block();
        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]).await;
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };
        assert!(size_bytes + offset_bytes <= inode_data.size.size());
        let mut levels = inode_data.size.ptr_levels();
        if levels == 0 {
            self.partition.read(inode_block_index as usize * 8 + 1, 7, buffer).await;
            unsafe { PAGE_TREE_ALLOCATOR.deallocate(inode_block_binding) };
            return;
        }
        //read first level pointers
        self.partition.read(inode_block_index as usize * 8 + 1, 7, &[inode_block]).await;

        let mut pointers: Vec<u32> = std::Vec::with_capacity(512 / 4 * 7);
        for i in (0..(512 * 7)).step_by(4) {
            pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + i) });
        }

        let mut first_ptr = 0;
        while levels > 1 {
            let ptr_span = 4096 + (levels - 1) * 1024;
            let first_relevant = (offset_bytes - first_ptr) / ptr_span;
            let last_relevant = (offset_bytes + aligned_size - 1 - first_ptr) / ptr_span;
            first_ptr += first_relevant * ptr_span;

            let mut new_pointers = std::Vec::with_capacity((last_relevant - first_relevant + 1) as usize * 1024);
            for i in first_relevant..=last_relevant {
                self.partition.read(pointers[i as usize] as usize * 8, 8, &[inode_block]).await;
                for i in (0..4096).step_by(4) {
                    new_pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + i) });
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
            let buf_index = i - first_relevant as usize;
            self.partition.read(
                pointers[i] as usize * BLOCK_SIZE_SECTORS,
                BLOCK_SIZE_SECTORS,
                &buffer[buf_index..=buf_index],
            ).await;
        }
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(inode_block_binding) };
    }

    pub async fn write_locked(&self, inode: InodeIndex, offset: u64, size: u64, buffer: &[PhysAddr]) -> vfs::Inode {
        let inode = inode as u32;
        assert!(offset % 4096 == 0);
        assert!(size.div_ceil(4096) <= buffer.len() as u64);
        //get info about file currently

        let inode_lock = self.inode_lock.lock().await;

        let root = unsafe { self.get_node(self.root_block).await.1 };
        let inode_block_index = BtreeNode::find_inode_block(root, inode, self).await.unwrap();
        drop(inode_lock); //file lock is held, so file won't move. Found the block
        let (inode_block, inode_block_binding) = get_working_block();
        self.partition.read(inode_block_index as usize * 8, 8, &[inode_block]).await;
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };

        let size_curr = inode_data.size.size();
        let size_new = u64::max(offset + size, size_curr);
        if size_new > size_curr {
            self.increase_file_size(inode_block_binding, inode_block, inode_block_index, size_new).await;
        }

        self.partition
            .read(inode_block_index as usize * BLOCK_SIZE_SECTORS, 8, &[inode_block]).await;
        //create a new reference to avoid rustc optimization issues. This is really a no-op anyway
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };

        let vfs_inode = inode_data.to_vfs(inode, &self.partition.partition);

        let mut levels = inode_data.size.ptr_levels();

        //Root now contains 1 pointer, to possibly data, table of pointers or a single pointer
        //to... if increased by >1 level

        if levels == 0 {
            assert!(size <= 512 * 7);
            self.partition.write(inode_block_index as usize * 8 + 1, 7, buffer).await;
            self.partition.write(inode_block_index as usize * 8, 1, &[inode_block]).await;
            unsafe { PAGE_TREE_ALLOCATOR.deallocate(inode_block_binding) };
            return vfs_inode;
        }

        let mut pointers: Vec<u32> = std::Vec::new();
        for i in (0..(512 * 7)).step_by(4) {
            pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + 512 + i) });
        }

        let aligned_size = size.div_ceil(4096) * 4096;

        let mut first_ptr = 0;
        while levels > 1 {
            let ptr_span = 4096 + (levels - 1) * 1024;
            let first_relevant = (offset - first_ptr) / ptr_span;
            let last_relevant = (offset + aligned_size - 1 - first_ptr) / ptr_span;
            first_ptr += first_relevant * ptr_span;

            let mut new_pointers =
                std::Vec::with_capacity((pointers.len() - (last_relevant - first_relevant + 1) as usize) * 1024);
            for i in first_relevant..=last_relevant {
                self.partition
                    .read(pointers[i as usize] as usize * BLOCK_SIZE_SECTORS, 8, &[inode_block]).await;
                for i in 0..1024 {
                    new_pointers.push(unsafe { *get_at_virtual_addr(inode_block_binding + i * 4) });
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
            let buffer_index = i - first_relevant as usize;
            self.partition.write(
                pointers[i] as usize * BLOCK_SIZE_SECTORS,
                8,
                &buffer[buffer_index..=buffer_index],
            ).await;
        }
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(inode_block_binding) };

        vfs_inode
    }

    async fn link_locked(&self, inode_index: InodeIndex, parent_inode_index: InodeIndex, name: &str) -> vfs::Inode {
        //TODO: i don't increase link count ??

        let inode_lock = self.inode_lock.lock().await;

        let root = unsafe { self.get_node(self.root_block).await.1 };
        let (working_block, working_block_binding) = get_working_block();
        let parent_inode_block_index = BtreeNode::find_inode_block(root, parent_inode_index as u32, self).await.unwrap();
        drop(inode_lock); //file lock is held, so file won't move. Found the block
        self.partition
            .read(parent_inode_block_index as usize * BLOCK_SIZE_SECTORS, 1, &[working_block]).await;
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(working_block_binding) };
        let offset = inode_data.size.size();

        let needs_second_block = (offset + core::mem::size_of::<DirEntry>() as u64) % 4096 < (offset % 4096);
        let (second_block, second_block_binding);
        if needs_second_block {
            (second_block, second_block_binding) = get_working_block();
        } else {
            second_block = PhysAddr(0);
            second_block_binding = VirtAddr(0);
        }

        if offset % 4096 != 0 {
            unsafe { self.read_locked(
                parent_inode_index,
                offset & (!0xFFF),
                u64::min(4096, inode_data.size.size()),
                &[working_block],
            ).await };
        }
        let name_bytes = name.as_bytes();
        let mut name_byte_arr: [u8; 128] = [0; 128];
        for char in name_bytes.iter().enumerate() {
            name_byte_arr[char.0] = *char.1;
        }
        let temp_offset = offset & 0xFFF;
        let dir_entry = DirEntry {
            inode: inode_index as u32,
            name: name_byte_arr,
        };
        let dir_entry_bytes = &dir_entry as *const DirEntry as *const u8;

        for i in 0..core::mem::size_of::<DirEntry>() {
            let new_off = temp_offset + i as u64;
            if new_off < 4096 {
                unsafe { set_at_virtual_addr(working_block_binding + new_off, dir_entry_bytes.add(i).read()) }
            } else {
                assert!(needs_second_block);
                unsafe { set_at_virtual_addr(second_block_binding + new_off % 4096, dir_entry_bytes.add(i).read()) }
            }
        }

        let write_size = if needs_second_block {
            8192
        } else {
            //could do 4096, but this allows small dirs do have their content in the same block as
            //the inode. This is unneeded for anything over 7 sectors, so
            offset + core::mem::size_of::<DirEntry>() as u64
        };
        let buffers: &[PhysAddr] = if needs_second_block {
            &[working_block, second_block]
        } else {
            &[working_block]
        };
        let vfs_inode = self.write_locked(parent_inode_index, offset & (!0xFFF), write_size, buffers).await;

        if needs_second_block {
            unsafe { PAGE_TREE_ALLOCATOR.deallocate(second_block_binding) };
        }
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(working_block_binding) };
        vfs_inode
    }

}

#[async_trait::async_trait]
impl FileSystem for Rfs {
    async fn unmount(&self) {
        self.clean_inode_tree_cache().await;
    }

    async fn read(&self, inode: InodeIndex, offset_bytes: u64, size_bytes: u64, buffer: &[PhysAddr]) {
        let file_lock = self.get_file_lock(inode as u32);
        let _read_guard = file_lock.lock_read().await;
        unsafe { self.read_locked(inode, offset_bytes, size_bytes, buffer).await };
        drop(_read_guard);
    }

    async fn write(&self, inode: InodeIndex, offset: u64, size: u64, buffer: &[PhysAddr]) -> vfs::Inode {
        let file_lock = self.get_file_lock(inode as u32);
        let _write_guard = file_lock.lock_write().await;
        let vfs_inode = self.write_locked(inode, offset, size, buffer).await;
        drop(_write_guard);
        vfs_inode
    }

    async fn stat(&self, inode: InodeIndex) -> crate::vfs::Inode {
        let inode = inode as u32;
        let inode_lock = self.inode_lock.lock().await;
        let root = unsafe { self.get_node(self.root_block).await.1 };
        let inode_block_index = BtreeNode::find_inode_block(root, inode, self).await.unwrap();
        drop(inode_lock);
        let (inode_block, inode_block_binding) = get_working_block();
        //no need to get file lock since this doesn't move
        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]).await;
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };
        let vfs_inode = inode_data.to_vfs(inode, &self.partition.partition);
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(inode_block_binding) };
        vfs_inode
    }

    async fn set_stat(&self, inode_index: InodeIndex, vfs_inode_data: vfs::Inode) {
        let inode_lock = self.inode_lock.lock().await;
        let root = unsafe { self.get_node(self.root_block).await.1 };
        let inode_block_index = BtreeNode::find_inode_block(root, inode_index as u32, self).await.unwrap();
        drop(inode_lock);
        let (inode_block, inode_block_binding) = get_working_block();
        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]).await;
        let inode_data: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };
        *inode_data = Inode::from_vfs(vfs_inode_data, inode_data.link_count, InodeSize(inode_data.size.0));
        //no need to get file lock since this doesn't move
        self.partition.write(inode_block_index as usize * 8, 1, &[inode_block]).await;
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(inode_block_binding) };
    }

    async fn create(
        &self,
        name: &str,
        parent_dir: InodeIndex,
        type_mode: crate::vfs::InodeType,
        uid: u16,
        gid: u16,
    ) -> (vfs::Inode, vfs::Inode) {
        let new_inode_block_index = self.allocate_block().await;
        let inode_lock = self.inode_lock.lock().await;
        let inode_index = unsafe { self.allocate_inode().await };
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
        let (inode_block, inode_block_binding) = get_working_block();
        let vfs_inode = inode.to_vfs(inode_index, &self.partition.partition);
        unsafe { set_at_virtual_addr(inode_block_binding, inode) };
        self.partition
            .write(new_inode_block_index as usize * 8, 1, &[inode_block])
            .await;

        let root = unsafe { self.get_node(self.root_block).await.1 };
        BtreeNode::insert_key_root(
            root,
            self.root_block,
            Key {
                index: inode_index,
                inode_block: new_inode_block_index,
            },
            self,
        ).await;
        let parent_lock = self.get_file_lock(parent_dir as u32);
        let new_lock = self.get_file_lock(inode_index);
        let _parent_guard = parent_lock.lock_write().await;
        let _new_guard = new_lock.lock_write().await;
        drop(inode_lock);

        let parent_vfs_inode = self.link_locked(inode_index as InodeIndex, parent_dir, name).await;

        drop(_new_guard);
        drop(_parent_guard);

        unsafe { PAGE_TREE_ALLOCATOR.deallocate(inode_block_binding) };

        (vfs_inode, parent_vfs_inode)
    }

    async fn unlink(&self, _parent_inode: InodeIndex, _name: &str) {
        todo!()
    }

    async fn link(&self, inode_index: InodeIndex, parent_inode_index: InodeIndex, name: &str) -> vfs::Inode {
        let parent_lock = self.get_file_lock(parent_inode_index as u32);
        let child_lock = self.get_file_lock(inode_index as u32);
        let _parent_guard = parent_lock.lock_write().await;
        let _child_guard = child_lock.lock_write().await;
        let vfs_inode = self.link_locked(inode_index, parent_inode_index, name).await;
        drop(_child_guard);
        drop(_parent_guard);
        vfs_inode
    }

    async fn truncate(&self, _inode: InodeIndex, _size: InodeIndex) {
        todo!()
    }

    async fn rename(&self, inode: InodeIndex, parent_inode: InodeIndex, name: &str) {
        let inode_lock = self.inode_lock.lock().await;

        let root = unsafe { self.get_node(self.root_block).await.1 };
        let parent_inode_block_index = BtreeNode::find_inode_block(root, parent_inode as u32, self).await.unwrap();
        drop(inode_lock); //file lock is held, so file won't move. Found the block
        let (working_block, working_block_binding) = get_working_block();

        let parent_lock = self.get_file_lock(parent_inode as u32);
        let _parent_guard = parent_lock.lock_write().await;

        self.partition
            .read(parent_inode_block_index as usize * 8, 1, &[working_block]).await;
        let parent_inode_data = unsafe { get_at_virtual_addr::<Inode>(working_block_binding) };
        let dir_size = parent_inode_data.size.size();
        let dir_block_count = dir_size.div_ceil(4096);

        let mut frames = Vec::with_capacity(dir_block_count as usize);
        for _ in 0..dir_block_count {
            frames.push(physical_allocator::allocate_frame());
        }
        let folder_binding = unsafe { PAGE_TREE_ALLOCATOR.mmap_contigious(&frames, false) };
        for i in 0..dir_block_count {
            unsafe {
                PAGE_TREE_ALLOCATOR
                    .get_page_table_entry_mut(folder_binding + i * 4096)
                    .unwrap()
                    .set_pat(LiminePat::UC);
            }
        }

        unsafe { self.read_locked(parent_inode, 0, dir_size, &frames).await };
        let mut affected_inode = 0;
        for i in 0..(dir_size / core::mem::size_of::<DirEntry>() as u64) {
            let dir_entry =
                unsafe { get_at_virtual_addr::<DirEntry>(folder_binding + i * core::mem::size_of::<DirEntry>() as u64) };
            if dir_entry.inode == inode as u32 {
                let name_bytes = name.as_bytes();
                let mut name_byte_arr: [u8; 128] = [0; 128];
                for char in name_bytes.iter().enumerate() {
                    name_byte_arr[char.0] = *char.1;
                }
                let mut new_dir_entry = dir_entry.clone();
                new_dir_entry.name = name_byte_arr;
                unsafe {
                    set_at_virtual_addr(folder_binding + i * core::mem::size_of::<DirEntry>() as u64, new_dir_entry);
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
        self.write(parent_inode, affected_block * 4096, write_size, buffers).await;
        drop(_parent_guard);

        for i in 0..dir_block_count {
            unsafe { PAGE_TREE_ALLOCATOR.deallocate(folder_binding + i * 4096) };
        }
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(folder_binding) };
        unsafe { PAGE_TREE_ALLOCATOR.deallocate(working_block_binding) };
    }

    async fn read_dir(&self, inode_index: InodeIndex) -> Box<[crate::drivers::disk::DirEntry]> {
        let inode_lock = self.inode_lock.lock().await;

        let root = unsafe { self.get_node(self.root_block).await.1 };
        let inode_block_index = BtreeNode::find_inode_block(root, inode_index as u32, self).await.unwrap();
        drop(inode_lock); //file lock is held, so file won't move. Found the block
        let (inode_block, inode_block_binding) = get_working_block();

        let file_lock = self.get_file_lock(inode_index as u32);
        let _file_guard = file_lock.lock_read().await;

        self.partition.read(inode_block_index as usize * 8, 1, &[inode_block]).await;
        let inode: &mut Inode = unsafe { get_at_virtual_addr(inode_block_binding) };

        let needed_blocks = inode.size.size().div_ceil(4096);
        if needed_blocks == 0 {
            return Box::new([]);
        }
        let phys_addresses = (0..needed_blocks)
            .map(|_| physical_allocator::allocate_frame())
            .collect::<Box<[_]>>();
        let virt_addr_start = unsafe { PAGE_TREE_ALLOCATOR.mmap_contigious(&phys_addresses, false) };
        unsafe { self.read_locked(inode_index, 0, inode.size.size(), &phys_addresses).await };
        drop(_file_guard);
        let mut entries = Vec::new();
        let mut offset = 0;
        while offset < inode.size.size() {
            let dir_entry = unsafe { get_at_virtual_addr::<DirEntry>(virt_addr_start + offset) };
            let name = str::from_utf8(&dir_entry.name).unwrap();
            let name = name.trim_matches('\0');
            let name = Box::from(name);
            entries.push(crate::drivers::disk::DirEntry {
                inode: dir_entry.inode as u64,
                name,
            });
            offset += core::mem::size_of::<DirEntry>() as u64;
        }

        entries.into_boxed_slice()
    }
}
