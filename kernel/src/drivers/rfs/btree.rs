use std::{mem_utils::VirtAddr, PAGE_ALLOCATOR};

use crate::{
    drivers::disk::Disk,
    memory::{paging, PAGE_TREE_ALLOCATOR},
};

use super::Rfs;

///Takes up exactly 1 block or physical frame
#[repr(C)]
#[derive(Debug)]
pub struct BtreeNode {
    keys: [(u32, u32); 341],
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

    fn get_key(self: *const Self, index: usize) -> (u32, u32) {
        unsafe { (self as *const (u32, u32)).add(index).read_volatile() }
    }

    fn set_key(self: *mut Self, index: usize, key: (u32, u32)) {
        unsafe {
            (self as *mut (u32, u32)).add(index).write_volatile(key);
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

    fn insert_key(self: *mut Self, block: u32, key: (u32, u32), fs_data: &mut Rfs) -> RebalanceResult {
        let is_leaf = self.get_child(0) == 0;
        if is_leaf {
            let is_full = self.get_key(340) != (0, 0);
            if is_full {
                let (new_block, separator) = self.insert_leaf_full(block, key, fs_data);
                return RebalanceResult::Split(new_block, separator);
            } else {
                self.insert_leaf_non_full(block, key, fs_data);
                return RebalanceResult::None;
            }
        }

        todo!()
    }

    fn insert_leaf_non_full(self: *mut Self, block: u32, key: (u32, u32), fs_data: &mut Rfs) {
        fs_data.get_node(block).0 = true;

        let mut ptr: i32 = 339;
        let key_inserted = false;
        while self.get_key(ptr as usize) == (0, 0) {
            ptr -= 1;
        }
        while ptr >= 0 && !key_inserted {
            let current_key = self.get_key(ptr as usize);
            if current_key > key {
                self.set_key(ptr as usize + 1, current_key);
                ptr -= 1;
            } else {
                self.set_key(ptr as usize + 1, key);
                return;
            }
        }
        self.set_key(0, key);
    }

    fn insert_leaf_full(self: *mut Self, block: u32, key: (u32, u32), fs_data: &mut Rfs) -> (u32, (u32, u32)) {
        //TODO: attempt rotation before split

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
                self.set_key(left_ptr as usize, (0, 0));
                right_ptr -= 1;
                left_ptr -= 1;
                continue;
            }
            let left_key = self.get_key(left_ptr as usize);
            if left_key > key {
                new_node.set_key(right_ptr as usize, left_key);
                self.set_key(left_ptr as usize, (0, 0));
                right_ptr -= 1;
                left_ptr -= 1;
            } else {
                self.set_key(right_ptr as usize, key);
                key_inserted = true;
                right_ptr -= 1;
            }
        }
        while !key_inserted && left_ptr >= 0 {
            let left_key = self.get_key(left_ptr as usize);
            if left_key > key {
                self.set_key(left_ptr as usize + 1, left_key);
                left_ptr -= 1;
            } else {
                self.set_key(left_ptr as usize + 1, key);
                key_inserted = true;
            }
        }
        if !key_inserted {
            self.set_key(0, key);
        }

        let separator = self.get_key(171);
        self.set_key(171, (0, 0));

        (new_block, separator)
    }

    fn rotate_left_take(self: *mut BtreeNode, block: u32, parent: u32, fs_data: &mut Rfs, leaf: bool) -> bool {
        let parent = fs_data.get_node(parent).1;
        let self_index = unsafe{&*parent}.children.iter().position(|&x| x == block).unwrap();
        let left_sibling = fs_data.get_node(unsafe{&*parent}.children[self_index - 1]);
        let left_key = unsafe{&*parent}.keys[self_index - 1];

        let sibling_has_elements = left_sibling.1.get_key(170) != (0, 0);
        if !sibling_has_elements {
            return false;
        }

        //shift self elements to the right. Self can't have more than 171 keys
        let mut ptr: i32 = 170;
        while ptr >= 0 {
            self.set_key(ptr as usize + 1, self.get_key(ptr as usize));
            ptr -= 1;
        }
        if !leaf {
            let mut ptr: i32 = 171;
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
            if left_sibling.1.get_key(i) != (0, 0) {
                last_key_index = i;
                break;
            }
        }

        //set parent's key to left sibling's last key
        unsafe{&mut *parent}.keys[self_index - 1] = left_sibling.1.get_key(last_key_index);

        //set self first child to left sibling's last child
        if !leaf {
            self.set_child(0, left_sibling.1.get_child(last_key_index + 1));
        }

        //remove left sibling's last key and child
        left_sibling.1.set_key(last_key_index, (0, 0));
        if !leaf {
            left_sibling.1.set_child(last_key_index + 1, 0);
        }

        true
    }
}


//rotations are done by children, not recorded here
enum RebalanceResult {
    ///Merged given into one being worked on. Parent should remove the key and child
    Merge(Direction),
    ///Rotate left or right. Child already does this, but parent should be saved
    Rotate,
    ///Always split left into right. u32 is the address, tuple is the key + value address
    Split(u32, (u32, u32)),
    None,
}

enum Direction {
    Left,
    Right,
}
