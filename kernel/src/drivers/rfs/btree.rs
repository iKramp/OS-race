use std::{mem_utils::VirtAddr, print, println, PAGE_ALLOCATOR};

use crate::{
    drivers::{disk::Disk, rfs::VIRTUAL_ONLY},
    memory::{paging, PAGE_TREE_ALLOCATOR},
};

//TODO:-------------CHECK FOR ALL USAGES OF SET_KEY AND GET_KEY AND ADD MODIFIED FLAG----------------

use super::Rfs;

///Takes up exactly 1 block or physical frame
#[repr(C)]
#[derive(Debug, Clone)]
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

    pub fn drop(self: *mut Self) {
        unsafe {
            PAGE_ALLOCATOR.deallocate(VirtAddr(self as u64));
        }
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

    pub fn print_inode_tree(block: u32, spaces: u32, fs_data: &mut Rfs) {
        if block == 1 {
            return;
        }
        let node = fs_data.get_node(block).1;
        let is_leaf = node.get_child(0) == 0;
        let mut max_key = 341;
        for i in 0..341 {
            if node.get_key(i).index == 0 {
                max_key = i;
                break;
            }
            if !is_leaf {
                Self::print_inode_tree(node.get_child(i), spaces + 1, fs_data);
            }
            let key = node.get_key(i).index;
            for _i in 0..spaces * 4 {
                print!(" ");
            }
            println!("{}", key);
        }
        if !is_leaf {
            Self::print_inode_tree(node.get_child(max_key), spaces + 1, fs_data);
        }
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
    pub fn insert_key_root(self: *mut Self, block: u32, key: Key, fs_data: &mut Rfs) -> Option<u32> {
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
                self.insert_non_full(block, key, None, fs_data);
                None
            }
        } else {
            //find first bigger key index
            for i in 0..341 {
                if key.index < self.get_key(i).index || self.get_key(i).index == 0 {
                    return self.insert_into_root_child(block, i, key, fs_data);
                }
            }
            self.insert_into_root_child(block, 341, key, fs_data)
        }
    }

    fn insert_into_root_child(self: *mut Self, block: u32, child_index: usize, key: Key, fs_data: &mut Rfs) -> Option<u32> {
        let is_full = self.get_key(340).index != 0;
        let child_node_index = self.get_child(child_index);
        let rebalance_result = fs_data
            .get_node(child_node_index)
            .1
            .insert_key_internal(child_node_index, block, key, fs_data);
        match rebalance_result {
            RebalanceResult::None => None,
            RebalanceResult::Merge(_) => {
                unreachable!("Nodes should not be merged when inserting keys");
            }
            RebalanceResult::Rotate(_) => {
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
        }
    }

    //returns a new root node if the root was merged
    pub fn delete_key_root(self: *mut Self, block: u32, key_index: u32, fs_data: &mut Rfs) -> Option<u32> {
        assert!(key_index > 2); //no deleting null keys, bad block file or root
        let is_leaf = self.get_child(0) == 0;

        if is_leaf {
            let mut deleted = false;
            for i in 0..340 {
                //will get overwritten, everything else past it will be shifted
                if key_index == self.get_key(i).index {
                    deleted = true;
                }
                if deleted {
                    self.set_key(i, self.get_key(i + 1));
                }
            }
            assert!(deleted);
            return None;
        }

        for i in 0..341 {
            if key_index == self.get_key(i).index {
                //child on the left of the key
                let child_block = self.get_child(i);
                let child_node = fs_data.get_node(child_block).1;
                let (key, rebalance_result) = child_node.take_biggest_key(child_block, block, fs_data);
                //If the result is a merge, this key has escaped to the merged child. Find and
                //replace it there. No need to worry about additional merges, as the node will be
                //full
                //if it is a rotate, it also disappeared somewhere
                match rebalance_result {
                    RebalanceResult::Merge(direction) => {
                        if matches!(direction, MergeDirection::RightToCurrent) {
                            for i in 0..341 {
                                if child_node.get_key(i).index == key_index {
                                    child_node.set_key(i, key);
                                    break;
                                }
                            }
                        } else {
                            child_node.set_key(i - 1, key);
                        }

                        if self.get_key(0).index == 0 {
                            //root is empty, merge
                            fs_data.remove_node(block);
                            fs_data.free_block(block);
                            let child = self.get_child(0);
                            self.drop();
                            return Some(child);
                        } else {
                            return None;
                        }
                    }
                    RebalanceResult::Rotate(direction) => {
                        if matches!(direction, RotateDirection::Left) {
                            for i in 160..180 {
                                if child_node.get_key(i).index == key_index {
                                    child_node.set_key(i, key);
                                    return None;
                                }
                            }
                            unreachable!("Key not found in child");
                        } else {
                            self.set_key(i, key);
                            return None;
                        }
                    }

                    RebalanceResult::Split(_, _) => unreachable!("Split should not happen when deleting keys"),
                    RebalanceResult::None => {
                        self.set_key(i, key);
                        return None;
                    }
                }
            } else if key_index < self.get_key(i).index || self.get_key(i).index == 0 {
                return self.delete_key_root_internal(block, key_index, i, fs_data);
            }
        }
        self.delete_key_root_internal(block, key_index, 341, fs_data)
    }

    fn delete_key_root_internal(
        self: *mut Self,
        block: u32,
        key_index: u32,
        child_index: usize,
        fs_data: &mut Rfs,
    ) -> Option<u32> {
        let child_block = self.get_child(child_index);
        let child_node = fs_data.get_node(child_block).1;
        let rebalance_result = child_node.delete_key_internal(child_block, key_index, block, fs_data);

        match rebalance_result {
            RebalanceResult::Merge(_) => {
                if self.get_key(0).index == 0 {
                    //root is empty, merge
                    fs_data.remove_node(block);
                    fs_data.free_block(block);
                    let child = self.get_child(0);
                    self.drop();
                    Some(child)
                } else {
                    None
                }
            }
            RebalanceResult::Rotate(_) => None,
            RebalanceResult::Split(_, _) => unreachable!("Split should not happen when deleting keys"),
            RebalanceResult::None => None,
        }
    }

    fn delete_key_internal(self: *mut Self, block: u32, key_index: u32, parent_block: u32, fs_data: &mut Rfs) -> RebalanceResult {
        let is_leaf = self.get_child(0) == 0;

        if is_leaf {
            let mut deleted = false;
            for i in 0..340 {
                //will get overwritten, everything else past it will be shifted
                if key_index == self.get_key(i).index {
                    deleted = true;
                }
                if deleted {
                    self.set_key(i, self.get_key(i + 1));
                }
            }
            assert!(deleted);
            let needs_rebalance = self.get_key(169).index == 0;
            if !needs_rebalance {
                return RebalanceResult::None;
            }
            let mut left = false;
            let mut result = self.rotate_left_take(block, parent_block, fs_data, true);
            if !result {
                left = true;
                result = self.rotate_right_take(block, parent_block, fs_data, true);
            }
            if result {
                return RebalanceResult::Rotate(if left { RotateDirection::Left } else { RotateDirection::Right });
            }
            let direction = self.merge(block, parent_block, fs_data);
            return RebalanceResult::Merge(direction);
        }

        for i in 0..341 {
            if key_index == self.get_key(i).index {
                //child on the left of the key
                let child_block = self.get_child(i);
                let child_node = fs_data.get_node(child_block).1;
                let (key, rebalance_result) = child_node.take_biggest_key(child_block, block, fs_data);
                //If the result is a merge, this key has escaped to the merged child. Find and
                //replace it there. No need to worry about additional merges, as the node will be
                //full
                //if it is a rotate, it also disappeared somewhere
                match rebalance_result {
                    RebalanceResult::Merge(direction) => {
                        if matches!(direction, MergeDirection::RightToCurrent) {
                            for i in 0..341 {
                                if child_node.get_key(i).index == key_index {
                                    child_node.set_key(i, key);
                                    break;
                                }
                            }
                        } else {
                            child_node.set_key(i - 1, key);
                        }

                        if self.get_key(169).index == 0 {
                            //Node is too small, fix

                            let mut left = false;
                            let mut result = self.rotate_left_take(block, parent_block, fs_data, false);
                            if !result {
                                left = true;
                                result = self.rotate_right_take(block, parent_block, fs_data, false);
                            }
                            if result {
                                return RebalanceResult::Rotate(if left { RotateDirection::Left } else { RotateDirection::Right });
                            }

                            let direction = self.merge(block, parent_block, fs_data);
                            return RebalanceResult::Merge(direction);
                        }
                    }
                    RebalanceResult::Rotate(direction) => {
                        if matches!(direction, RotateDirection::Left) {
                            for i in 160..341 {
                                if self.get_key(i).index == key_index {
                                    self.set_key(i, key);
                                    return RebalanceResult::None;
                                }
                            }
                        } else {
                            self.set_key(i, key);
                            return RebalanceResult::None;
                        }
                    }

                    RebalanceResult::Split(_, _) => unreachable!("Split should not happen when deleting keys"),
                    RebalanceResult::None => {
                        self.set_key(i, key);
                        return RebalanceResult::None;
                    }
                }
                unreachable!();
            } else if key_index < self.get_key(i).index {
                return self.delete_key_internal(block, key_index, i as u32, fs_data);
            }
        }
        self.delete_key_internal(block, key_index, 341, fs_data)
    }

    fn take_biggest_key(self: *mut Self, block: u32, parent_block: u32, fs_data: &mut Rfs) -> (Key, RebalanceResult) {
        //this code is almost identical to the delete_key_internal
        let is_leaf = self.get_child(0) == 0;
        if is_leaf {
            let mut index = 0;
            for i in (0..341).rev() {
                if self.get_key(i).index != 0 {
                    index = i;
                    break;
                }
            }
            if index < 169 {
                assert!(index == 168);
                let key = self.get_key(index);
                self.set_key(index, Key::empty());
                //we need to rebalance
                let mut left = false;
                let mut result = self.rotate_left_take(block, parent_block, fs_data, true);
                if !result {
                    left = true;
                    result = self.rotate_right_take(block, parent_block, fs_data, true);
                }
                index += 1;

                if result {
                    let key = self.get_key(index);
                    self.set_key(index, Key::empty());
                    return (
                        key,
                        RebalanceResult::Rotate(if left { RotateDirection::Left } else { RotateDirection::Right }),
                    );
                } else {
                    //merge is needed
                    let direction = self.merge(block, parent_block, fs_data);
                    return (key, RebalanceResult::Merge(direction));
                }
            }
            let key = self.get_key(index);
            self.set_key(index, Key::empty());
            return (key, RebalanceResult::None);
        }

        for i in (0..341).rev() {
            if self.get_key(i).index != 0 {
                let child_index = i + 1;
                return self.take_biggest_from_child(block, child_index, parent_block, fs_data);
            }
        }
        unreachable!("This should never happen, as the node would have to have 0 children");
    }

    fn take_biggest_from_child(
        self: *mut BtreeNode,
        block: u32,
        child_index: usize,
        parent_block: u32,
        fs_data: &mut Rfs,
    ) -> (Key, RebalanceResult) {
        let child_node_block = self.get_child(child_index);
        let (key, rebalance_result) = fs_data
            .get_node(child_node_block)
            .1
            .take_biggest_key(child_node_block, block, fs_data);
        match rebalance_result {
            RebalanceResult::Merge(_direction) => {
                let mut last_key = 0;
                for i in (0..342).rev() {
                    if self.get_key(i).index != 0 {
                        last_key = i;
                        break;
                    }
                }
                if last_key > 169 {
                    //is still balanced
                    return (key, RebalanceResult::None);
                }
                //rotate
                let mut result = self.rotate_left_give(block, parent_block, fs_data, false);
                if !result {
                    result = self.rotate_right_give(block, parent_block, fs_data, false);
                }
                if result {
                    return (key, RebalanceResult::None);
                }

                let direction = self.merge(block, parent_block, fs_data);
                (key, RebalanceResult::Merge(direction))
            }
            RebalanceResult::Rotate(_) => (key, RebalanceResult::None),
            RebalanceResult::Split(_, _) => unreachable!("Split should not happen when taking keys"),
            RebalanceResult::None => (key, RebalanceResult::None),
        }
    }

    fn merge(self: *mut Self, block: u32, parent_block: u32, fs_data: &mut Rfs) -> MergeDirection {
        let parent = fs_data.get_node(parent_block).1;
        let self_index = unsafe { &*parent }.children.iter().position(|&x| x == block).unwrap();
        let (left_node, right_node, separator, direction, right_block);
        if self_index == 0 {
            left_node = self;
            right_block = unsafe { &*parent }.children[self_index + 1];
            right_node = fs_data.get_node(right_block).1;
            separator = unsafe { &*parent }.keys[self_index];
            direction = MergeDirection::RightToCurrent;
        } else {
            left_node = fs_data.get_node(unsafe { &*parent }.children[self_index - 1]).1;
            right_node = self;
            separator = unsafe { &*parent }.keys[self_index - 1];
            direction = MergeDirection::CurrentToLeft;
            right_block = block;
        }

        let mut key_index = 0;
        //it's somewhere in the middle idk exactly
        for i in (160..180).rev() {
            if left_node.get_key(i).index != 0 {
                key_index = i + 1;
                break;
            }
        }

        left_node.set_key(key_index, separator);
        key_index += 1;

        let mut right_ptr = 0;
        let mut right_key = right_node.get_key(right_ptr);
        while right_key.index != 0 {
            left_node.set_key(key_index, right_key);
            right_node.set_key(right_ptr, Key::empty());
            left_node.set_child(key_index, right_node.get_child(right_ptr));
            right_node.set_child(right_ptr, 0);
            key_index += 1;
            right_ptr += 1;
            right_key = right_node.get_key(right_ptr);
        }

        let shift_ptr = if self_index == 0 { 1 } else { self_index };
        for i in shift_ptr..341 {
            parent.set_key(i, parent.get_key(i + 1));
            parent.set_child(i + 1, parent.get_child(i + 2));
        }

        parent.set_key(341, Key::empty());
        parent.set_child(342, 0);

        right_node.drop();
        fs_data.free_block(right_block);
        fs_data.remove_node(right_block);

        direction
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
        self.set_key(170, Key::empty());

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
        for i in 0..341 {
            if key.index < self.get_key(i).index || self.get_key(i).index == 0 {
                return self.insert_into_child(block, i, parent_block, key, fs_data);
            }
        }
        self.insert_into_child(block, 341, parent_block, key, fs_data)
    }

    fn insert_into_child(
        self: *mut Self,
        block: u32,
        child_index: usize,
        parent_block: u32,
        key: Key,
        fs_data: &mut Rfs,
    ) -> RebalanceResult {
        let child_node_index = self.get_child(child_index);
        let rebalance_result = fs_data
            .get_node(child_node_index)
            .1
            .insert_key_internal(child_node_index, block, key, fs_data);
        match rebalance_result {
            RebalanceResult::None => RebalanceResult::None,
            RebalanceResult::Merge(_) => {
                unreachable!("Nodes should not be merged when inserting keys");
            }
            RebalanceResult::Rotate(_) => {
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
        }
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

    ///Child must be on the right of the key
    fn insert_full(
        self: *mut Self,
        block: u32,
        parent_block: u32,
        key: Key,
        child: Option<u32>,
        fs_data: &mut Rfs,
    ) -> RebalanceResult {
        let mut left = true;
        let mut result = self.rotate_left_give(block, parent_block, fs_data, child.is_none());
        if !result {
            left = false;
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
            return RebalanceResult::Rotate(if left { RotateDirection::Left } else { RotateDirection::Right });
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
        if self_index == 0 {
            return false;
        }
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

        let mut last_key_index = 340;
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
        if unsafe { *(*parent).children.get_unchecked(self_index + 1) } == 0 {
            return false;
        }
        let right_sibling = fs_data.get_node(unsafe { &*parent }.children[self_index + 1]);
        let right_key = parent.get_key(self_index);

        let sibling_has_elements = right_sibling.1.get_key(170).index != 0;
        let self_has_space = self.get_key(340).index == 0;
        if !sibling_has_elements || !self_has_space {
            return false;
        }

        let mut last_key_index = 0;
        //find where self's last key is
        for i in (0..340).rev() {
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
        parent.set_key(self_index, right_sibling.1.get_key(0));

        //shift right sibling's elements to the left
        let mut ptr: i32 = 0;
        while ptr < 340 {
            right_sibling
                .1
                .set_key(ptr as usize, right_sibling.1.get_key(ptr as usize + 1));
            ptr += 1;
        }
        if !leaf {
            let mut ptr: i32 = 0;
            while ptr < 341 {
                right_sibling
                    .1
                    .set_child(ptr as usize, right_sibling.1.get_child(ptr as usize + 1));
                ptr += 1;
            }
        }
        right_sibling.1.set_key(340, Key::empty());
        if !leaf {
            right_sibling.1.set_child(341, 0);
        }

        right_sibling.0 = true;
        fs_data.get_node(block).0 = true;
        fs_data.get_node(parent_block).0 = true;

        true
    }

    fn rotate_left_give(self: *mut BtreeNode, block: u32, parent_block: u32, fs_data: &mut Rfs, leaf: bool) -> bool {
        let parent = fs_data.get_node(parent_block).1;
        let self_index = unsafe { &*parent }.children.iter().position(|&x| x == block).unwrap();
        if self_index == 0 {
            return false;
        }
        let left_sibling_block = unsafe { &*parent }.children[self_index - 1];
        let left_sibling = fs_data.get_node(left_sibling_block);
        left_sibling
            .1
            .rotate_right_take(left_sibling_block, parent_block, fs_data, leaf)
    }

    fn rotate_right_give(self: *mut BtreeNode, block: u32, parent_block: u32, fs_data: &mut Rfs, leaf: bool) -> bool {
        let parent = fs_data.get_node(parent_block).1;
        let self_index = unsafe { &*parent }.children.iter().position(|&x| x == block).unwrap();
        if self_index == 341 || unsafe { *(*parent).children.get_unchecked(self_index + 1) } == 0 {
            return false;
        }
        let right_sibling_block = unsafe { &*parent }.children[self_index + 1];
        let right_sibling = fs_data.get_node(right_sibling_block);
        right_sibling
            .1
            .rotate_right_take(right_sibling_block, parent_block, fs_data, leaf)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Key {
    pub index: u32,
    pub indoe_block: u32,
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
    ///always merge right into left so left doesn't have to be rearranged. Children should merge
    Merge(MergeDirection),
    ///Rotate left or right. Child already does this, but parent should be saved
    Rotate(RotateDirection),
    ///Always split left into right. u32 is the address of the right block, tuple is the key + value address
    Split(u32, Key),
    None,
}

enum MergeDirection {
    RightToCurrent,
    CurrentToLeft,
}

enum RotateDirection {
    Left,
    Right,
}
