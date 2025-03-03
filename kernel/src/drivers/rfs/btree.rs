use std::{PAGE_ALLOCATOR, mem_utils::VirtAddr};

use crate::{
    drivers::disk::Disk,
    memory::{PAGE_TREE_ALLOCATOR, paging},
};

use super::Rfs;

///Takes up exactly 1 block or physical frame
#[repr(C)]
#[derive(Debug)]
pub struct BtreeNode {
    keys: [Key; 341],
    children: [u32; 342],
}

impl BtreeNode {
    pub fn read_from_disk(disk: &mut dyn Disk, block: u32) -> *mut Self {
        let sector = block as usize * 8;

        let virt_ptr = unsafe { PAGE_ALLOCATOR.allocate(None) };
        unsafe {
            PAGE_TREE_ALLOCATOR
                .get_page_table_entry_mut(virt_ptr)
                .set_pat(paging::LiminePat::UC);
        }
        let command_slot = disk.read(sector, 8, virt_ptr);
        disk.clean_after_read(command_slot);
        unsafe { &mut *(virt_ptr.0 as *mut BtreeNode) }
    }

    ///set modified to false
    fn write_to_disk(self: *const Self, disk: &mut dyn Disk, block: u32) {
        let sector = block as usize * 8;

        let command_slot = disk.write(sector, 8, VirtAddr(self as u64));
        disk.clean_after_write(command_slot);
    }

    fn new() -> *mut Self {
        let virt_ptr = unsafe { PAGE_ALLOCATOR.allocate(None) };
        unsafe {
            std::mem_utils::memset_virtual_addr(virt_ptr, 0, 4096);
        }
        virt_ptr.0 as *mut BtreeNode
    }

    fn get_key(self: *const Self, index: usize) -> Key {
        unsafe { (self as *const Key).add(index).read_volatile() }
    }

    fn set_key(self: *mut Self, index: usize, key: Key) {
        unsafe {
            (self as *mut Key).add(index).write_volatile(key);
        }
    }

    fn get_child(self: *const Self, index: usize) -> u32 {
        unsafe { (self as *const u32).byte_add(0xAA8).add(index).read_volatile() }
    }

    fn set_child(self: *mut Self, index: usize, child: u32) {
        unsafe {
            (self as *mut u32).byte_add(0xAA8).add(index).write_volatile(child);
        }
    }

    //returns a new root node if the root was split
    fn insert_key_root(self: *mut Self, block: u32, key: Key, fs_data: &mut Rfs) -> Option<u32> {
        let is_leaf = self.get_child(0) == 0;
        let is_full = self.get_key(340).index != 0;

        if is_leaf {
            if is_full {
                let new_root_block = self.split_root(block, fs_data);
                let new_root_node = fs_data.get_node(new_root_block).1;
                if key.index < new_root_node.get_key(0).index {
                    new_root_node.insert_key_internal(new_root_block, 0, key, fs_data);
                } else {
                    new_root_node.insert_key_internal(new_root_block, block, key, fs_data);
                }
                Some(new_root_block)
            } else {
                //find first bigger key index
                self.insert_non_full(block, key, None, fs_data);
                None
            }
        } else {
            //find first bigger key index
            for i in 0..340 {
                if key.index < self.get_key(i).index {
                    let child_node_index = self.get_child(i);
                    let rebalance_result =
                        fs_data
                            .get_node(child_node_index)
                            .1
                            .insert_key_internal(child_node_index, block, key, fs_data);
                    return match rebalance_result {
                        RebalanceResult::None => None,
                        RebalanceResult::Merge(_direction) => {
                            unreachable!("Nodes should not be merged when inserting keys");
                        }
                        RebalanceResult::Rotate => {
                            //child does everything
                            None
                        }
                        RebalanceResult::Split(new_block, new_key) => {
                            if is_full {
                                let new_root_block = self.split_root(block, fs_data);
                                let new_root_node = fs_data.get_node(new_root_block).1;
                                if new_key.index < new_root_node.get_key(0).index {
                                    new_root_node.insert_key_internal(new_root_block, 0, new_key, fs_data);
                                } else {
                                    new_root_node.insert_key_internal(new_root_block, block, new_key, fs_data);
                                }
                                Some(new_root_block)
                            } else {
                                self.insert_non_full(block, new_key, Some(new_block), fs_data);
                                None
                            }
                        }
                    };
                }
            }
            unreachable!("key was inserted into an empty node??");
        }
    }

    fn insert_key_internal(self: *mut Self, block: u32, parent_block: u32, key: Key, fs_data: &mut Rfs) -> RebalanceResult {
        let is_leaf = self.get_child(0) == 0;
        if is_leaf {
            let is_full = self.get_key(340).index != 0;
            if is_full {
                return self.insert_full(block, parent_block, key, None, fs_data);
            } else {
                self.insert_non_full(block, key, None, fs_data);
                return RebalanceResult::None;
            }
        }
        //find first bigger key index
        for i in 0..340 {
            if key.index < self.get_key(i).index {
                let child_node_index = self.get_child(i);
                let rebalance_result =
                    fs_data
                        .get_node(child_node_index)
                        .1
                        .insert_key_internal(child_node_index, block, key, fs_data);
                return match rebalance_result {
                    RebalanceResult::None => RebalanceResult::None,
                    RebalanceResult::Merge(_direction) => {
                        unreachable!("Nodes should not be merged when inserting keys");
                    }
                    RebalanceResult::Rotate => {
                        //child does everything
                        RebalanceResult::None
                    }
                    RebalanceResult::Split(new_block, new_key) => {
                        let self_full = self.get_key(340).index != 0;
                        if self_full {
                            self.insert_full(block, parent_block, new_key, Some(new_block), fs_data)
                        } else {
                            self.insert_non_full(block, new_key, Some(new_block), fs_data);
                            RebalanceResult::None
                        }
                    }
                };
            }
        }
        unreachable!("key was inserted into an empty node??");
    }

    fn insert_non_full(self: *mut Self, block: u32, key: Key, child: Option<u32>, fs_data: &mut Rfs) {
        fs_data.get_node(block).0 = true;

        let mut ptr: i32 = 339;
        let key_inserted = false;
        while self.get_key(ptr as usize).index == 0 {
            ptr -= 1;
        }
        while ptr >= 0 && !key_inserted {
            let current_key = self.get_key(ptr as usize);
            if current_key.index > key.index {
                self.set_key(ptr as usize + 1, current_key);
                self.set_child(ptr as usize + 2, self.get_child(ptr as usize + 1));
                ptr -= 1;
            } else {
                self.set_key(ptr as usize + 1, key);
                self.set_child(ptr as usize + 2, child.unwrap_or(0));
                return;
            }
        }
        self.set_key(0, key);
        self.set_child(1, child.unwrap_or(0));
    }

    //returns the new root
    fn split_root(self: *mut Self, block: u32, fs_data: &mut Rfs) -> u32 {
        let sibling_block = fs_data.allocate_block();
        let parent_block = fs_data.allocate_block();
        let sibling_node = BtreeNode::new();
        let parent_node = BtreeNode::new();

        fs_data.add_node(sibling_block, sibling_node);
        fs_data.add_node(parent_block, parent_node);

        let separator = self.get_key(170);

        for i in 171..341 {
            sibling_node.set_key(i - 171, self.get_key(i));
            sibling_node.set_child(i - 170, self.get_child(i + 1));
            self.set_key(i, Key::empty());
            self.set_child(i + 1, 0);
        }
        sibling_node.set_child(171, self.get_child(341));

        parent_node.set_key(0, separator);
        parent_node.set_child(0, block);
        parent_node.set_child(1, sibling_block);

        fs_data.get_node(block).0 = true;

        parent_block
    }

    ///Child must be on the right of the key
    fn insert_full(
        self: *mut Self,
        block: u32,
        parent_block: u32,
        key: Key,
        child: Option<u32>,
        fs_data: &mut Rfs,
    ) -> RebalanceResult {
        let mut result = self.rotate_left_give(block, parent_block, fs_data, child.is_none());
        if !result {
            result = self.rotate_right_give(block, parent_block, fs_data, child.is_none());
        }

        if result {
            //-------------------ROTATE SUCCESSFUL-------------------
            //find correct key
            for i in (0..340).rev() {
                let curr_key = self.get_key(i);
                if curr_key.index == 0 {
                    continue;
                }
                if curr_key.index < key.index {
                    self.set_key(i + 1, key);
                    self.set_child(i + 2, child.unwrap_or(0));
                    break;
                }
                self.set_key(i + 1, self.get_key(i));
                if child.is_some() {
                    self.set_child(i + 2, self.get_child(i + 1));
                }
            }
            return RebalanceResult::Rotate;
        }

        //-------------------SPLIT NODE-------------------
        let new_block = fs_data.allocate_block();
        let new_node = BtreeNode::new();

        fs_data.add_node(new_block, new_node);
        fs_data.get_node(block).0 = true;

        //copy half of the elements to the new node, but take care to insert the key when
        //necessary. One node has 341 keys. 170/171 after split
        let mut left_ptr: i32 = 340;
        let mut right_ptr: i32 = 169;
        let mut key_inserted = false;
        while right_ptr >= 0 {
            if key_inserted {
                new_node.set_key(right_ptr as usize, self.get_key(left_ptr as usize));
                new_node.set_child(right_ptr as usize + 1, self.get_child(left_ptr as usize + 1));
                self.set_key(left_ptr as usize, Key::empty());
                self.set_child(left_ptr as usize + 1, 0);
                right_ptr -= 1;
                left_ptr -= 1;
                continue;
            }
            let left_key = self.get_key(left_ptr as usize);
            if left_key.index > key.index {
                new_node.set_key(right_ptr as usize, left_key);
                new_node.set_child(right_ptr as usize + 1, self.get_child(left_ptr as usize + 1));
                self.set_key(left_ptr as usize, Key::empty());
                self.set_child(left_ptr as usize + 1, 0);
                right_ptr -= 1;
                left_ptr -= 1;
            } else {
                new_node.set_key(right_ptr as usize, key);
                new_node.set_child(right_ptr as usize + 1, child.unwrap_or(0));
                key_inserted = true;
                right_ptr -= 1;
            }
        }
        while !key_inserted && left_ptr >= 0 {
            let left_key = self.get_key(left_ptr as usize);
            if left_key.index > key.index {
                self.set_key(left_ptr as usize + 1, left_key);
                self.set_child(left_ptr as usize + 2, self.get_child(left_ptr as usize + 1));
                left_ptr -= 1;
            } else {
                self.set_key(left_ptr as usize + 1, key);
                self.set_child(left_ptr as usize + 2, child.unwrap_or(0));
                key_inserted = true;
            }
        }
        if !key_inserted {
            self.set_key(0, key);
            self.set_child(1, child.unwrap_or(0));
        }

        let separator = self.get_key(171);
        self.set_key(171, Key::empty());

        RebalanceResult::Split(new_block, separator)
    }

    fn rotate_left_take(self: *mut BtreeNode, block: u32, parent_block: u32, fs_data: &mut Rfs, leaf: bool) -> bool {
        let parent = fs_data.get_node(parent_block).1;
        let self_index = unsafe { &*parent }.children.iter().position(|&x| x == block).unwrap();
        let left_sibling = fs_data.get_node(unsafe { &*parent }.children[self_index - 1]);
        let left_key = unsafe { &*parent }.keys[self_index - 1];

        let sibling_has_elements = left_sibling.1.get_key(170).index != 0;
        let self_has_space = self.get_key(340).index == 0;
        if !sibling_has_elements || !self_has_space {
            return false;
        }

        //shift self elements to the right
        let mut ptr: i32 = 339;
        while ptr >= 0 {
            self.set_key(ptr as usize + 1, self.get_key(ptr as usize));
            ptr -= 1;
        }
        if !leaf {
            let mut ptr: i32 = 340;
            while ptr >= 0 {
                self.set_child(ptr as usize + 1, self.get_child(ptr as usize));
                ptr -= 1;
            }
        }

        //insert the key from the parent
        self.set_key(0, left_key);

        let mut last_key_index = 0;
        //find where left sibling's last key is
        for i in (0..340).rev() {
            if left_sibling.1.get_key(i).index != 0 {
                last_key_index = i;
                break;
            }
        }

        //set parent's key to left sibling's last key
        unsafe { &mut *parent }.keys[self_index - 1] = left_sibling.1.get_key(last_key_index);

        //set self first child to left sibling's last child
        if !leaf {
            self.set_child(0, left_sibling.1.get_child(last_key_index + 1));
        }

        //remove left sibling's last key and child
        left_sibling.1.set_key(last_key_index, Key::empty());
        if !leaf {
            left_sibling.1.set_child(last_key_index + 1, 0);
        }

        left_sibling.0 = true;
        fs_data.get_node(block).0 = true;
        fs_data.get_node(parent_block).0 = true;

        true
    }

    fn rotate_right_take(self: *mut BtreeNode, block: u32, parent_block: u32, fs_data: &mut Rfs, leaf: bool) -> bool {
        let parent = fs_data.get_node(parent_block).1;
        let self_index = unsafe { &*parent }.children.iter().position(|&x| x == block).unwrap();
        let right_sibling = fs_data.get_node(unsafe { &*parent }.children[self_index + 1]);
        let right_key = unsafe { &*parent }.keys[self_index];

        let sibling_has_elements = right_sibling.1.get_key(170).index != 0;
        let self_has_space = self.get_key(340).index == 0;
        if !sibling_has_elements || !self_has_space {
            return false;
        }

        let mut last_key_index = 0;
        //find where self's last key is
        for i in 0..340 {
            if self.get_key(i).index != 0 {
                last_key_index = i;
                break;
            }
        }

        //insert the key from the parent
        self.set_key(last_key_index + 1, right_key);

        //set self last child to right sibling's first child
        if !leaf {
            self.set_child(last_key_index + 2, right_sibling.1.get_child(0));
        }

        //set parent's key to right sibling's first key
        unsafe { &mut *parent }.keys[self_index] = right_sibling.1.get_key(0);

        //shift right sibling's elements to the left
        let mut ptr: i32 = 0;
        while ptr < 339 {
            right_sibling
                .1
                .set_key(ptr as usize, right_sibling.1.get_key(ptr as usize + 1));
            ptr += 1;
        }
        if !leaf {
            let mut ptr: i32 = 0;
            while ptr < 340 {
                right_sibling
                    .1
                    .set_child(ptr as usize, right_sibling.1.get_child(ptr as usize + 1));
                ptr += 1;
            }
        }

        right_sibling.0 = true;
        fs_data.get_node(block).0 = true;
        fs_data.get_node(parent_block).0 = true;

        true
    }

    fn rotate_left_give(self: *mut BtreeNode, block: u32, parent_block: u32, fs_data: &mut Rfs, leaf: bool) -> bool {
        let parent = fs_data.get_node(parent_block).1;
        let self_index = unsafe { &*parent }.children.iter().position(|&x| x == block).unwrap();
        let left_sibling_block = unsafe { &*parent }.children[self_index - 1];
        let left_sibling = fs_data.get_node(left_sibling_block);
        left_sibling
            .1
            .rotate_right_take(left_sibling_block, parent_block, fs_data, leaf)
    }

    fn rotate_right_give(self: *mut BtreeNode, block: u32, parent_block: u32, fs_data: &mut Rfs, leaf: bool) -> bool {
        let parent = fs_data.get_node(parent_block).1;
        let self_index = unsafe { &*parent }.children.iter().position(|&x| x == block).unwrap();
        let right_sibling_block = unsafe { &*parent }.children[self_index + 1];
        let right_sibling = fs_data.get_node(right_sibling_block);
        right_sibling
            .1
            .rotate_right_take(right_sibling_block, parent_block, fs_data, leaf)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Key {
    index: u32,
    indoe_block: u32,
}

impl Key {
    fn empty() -> Self {
        Self {
            index: 0,
            indoe_block: 0,
        }
    }
}

//rotations are done by children, not recorded here
enum RebalanceResult {
    ///Merged given into one being worked on. Parent should remove the key and child
    Merge(Direction),
    ///Rotate left or right. Child already does this, but parent should be saved
    Rotate,
    ///Always split left into right. u32 is the address of the right block, tuple is the key + value address
    Split(u32, Key),
    None,
}

enum Direction {
    Left,
    Right,
}
