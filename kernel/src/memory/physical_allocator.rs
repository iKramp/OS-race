
use std::{eh::int3};

use crate::{limine, println};

use super::mem_utils::*;

pub static mut BUDDY_ALLOCATOR: BuyddyAllocator = BuyddyAllocator {
    n_pages: 0,
    binary_tree_size: 0,
    allocated_pages: 0,
    tree_allocator: VirtAddr(0),
};

pub struct BuyddyAllocator {
    pub n_pages: u64,
    binary_tree_size: u64,
    allocated_pages: u64,
    tree_allocator: VirtAddr,
}

impl BuyddyAllocator {
    pub fn init() {
        let memory_regions = unsafe { &mut *(*crate::LIMINE_BOOTLOADER_REQUESTS.memory_map_request.info).memory_map };
        let memory_regions = unsafe { core::slice::from_raw_parts_mut(memory_regions, (*crate::LIMINE_BOOTLOADER_REQUESTS.memory_map_request.info).memory_map_count as usize) };
        let n_pages = find_max_usable_address(memory_regions).0 >> 12;
        println!("n_pages: {}", n_pages);
        println!("max memory address: {:#X}", n_pages * 4096);

        let binary_tree_size_elements = get_binary_tree_size(n_pages);
        // div by 8 for 8 bits in a byte  (also rounded up), times 2 for binary tree
        let space_needed_bytes = (binary_tree_size_elements + 7) / 8;
        let entry_to_shrink = find_mem_region_to_shrink(memory_regions, space_needed_bytes);

        let tree_allocator = PhysAddr(memory_regions[entry_to_shrink].base);
        let mut allocated_pages = 0;
        for i in 0..space_needed_bytes {
            //set all bits to 1 to set everything as used
            unsafe {
                set_at_physical_addr(tree_allocator + PhysAddr(i), 0xFF_u8);
                allocated_pages += 4;
            }
        }
        let mut size_to_shrink = space_needed_bytes & !0xFFF;
        if space_needed_bytes & 0xFFF > 0 {
            size_to_shrink += 0x1000;
        }
        //we essentially round up to a whole page
        memory_regions[entry_to_shrink].base += size_to_shrink;
        let mut allocator = Self {
            n_pages,
            binary_tree_size: binary_tree_size_elements,
            allocated_pages,
            tree_allocator: translate_phys_virt_addr(tree_allocator),
        };
        for entry in memory_regions {
            if !is_memory_region_usable(entry) {
                continue;
            }
            let start = if entry.base & 0xFFF == 0 {
                entry.base
            } else {
                (entry.base & !0xFFF) + 0x1000
            };
            for addr in (start..((start + entry.length) & !0xFFF)).step_by(4096) {
                allocator.mark_addr(PhysAddr(addr), false);
                allocator.allocated_pages -= 1;
            }
        }
        unsafe { BUDDY_ALLOCATOR = allocator }
    }

    pub fn is_frame_allocated(&self, addr: PhysAddr) -> bool {
        #[cfg(debug_assertions)]
        assert!(
            addr.0 & 0xFFF == 0,
            "error in is_frame_allocated at addr {}: address is not page aligned",
            addr.0,
        );
        self.get_at_index((addr.0 >> 12) + (self.binary_tree_size / 2))
    }

    pub fn deallocate_frame(&mut self, addr: PhysAddr) {
        self.mark_addr(addr, false);
        self.allocated_pages -= 1;
    }

    pub fn allocate_frame(&mut self) -> PhysAddr {
        if self.allocated_pages == self.binary_tree_size / 2 {
            panic!("no more frames to sllocate");
        }
        self.allocated_pages += 1;
        let index = self.find_empty_frame();
        self.mark_index(index, true);
        let address = (index - self.binary_tree_size / 2) * 4096;
        debug_assert!(address <= self.n_pages * 4096, "address is out of bounds");
        PhysAddr(address)
    }

    fn find_empty_frame(&self) -> u64 {
        assert!(!self.get_at_index(1), "root node of physical memory allocator is filled");
        self.find_empty_frame_recursively(1)
    }

    fn find_empty_frame_recursively(&self, curr_index: u64) -> u64 {
        if curr_index >= self.binary_tree_size / 2 {
            //is in second half of the tree, so last level
            return curr_index;
        }
        #[cfg(debug_assertions)]
        assert!(
            !self.get_at_index(curr_index),
            "asked to find empty page from this index {} but all sub-regions are filled",
            curr_index
        );
        if !self.get_at_index(curr_index * 2) {
            self.find_empty_frame_recursively(curr_index * 2)
        } else {
            self.find_empty_frame_recursively(curr_index * 2 + 1)
        }
    }

    pub fn mark_addr(&self, addr: PhysAddr, allocated: bool) {
        #[cfg(debug_assertions)]
        assert!(
            addr.0 & 0xFFF == 0,
            "error in mark_addr at addr {} and allocated {}: address is not page aligned",
            addr.0,
            allocated
        );
        self.mark_index((addr.0 >> 12) + (self.binary_tree_size / 2), allocated);
    }

    fn mark_index(&self, index: u64, allocated: bool) {
        self.set_at_index(index, allocated);
        self.update_from_lower(index >> 1);
        self.update_from_higher(index << 1, allocated);
    }

    fn update_from_lower(&self, index: u64) {
        if index == 0 {
            return;
        }
        let all_allocated = self.get_at_index(index << 1) && self.get_at_index((index << 1) + 1);
        self.set_at_index(index, all_allocated);
        self.update_from_lower(index >> 1);
    }

    //should only be used when a page or group of pages is set, so all sub-fragments are also
    //marked as used/unused
    fn update_from_higher(&self, index: u64, allocated: bool) {
        if index >= self.n_pages + self.binary_tree_size / 2 {
            //out of bounds
            return;
        }
        self.set_at_index(index, allocated);

        self.update_from_higher(index << 1, allocated);
        self.update_from_higher((index << 1) + 1, allocated);
    }

    fn get_at_index(&self, index: u64) -> bool {
        unsafe {
            let num = *get_at_virtual_addr::<u8>(self.tree_allocator + VirtAddr(index >> 3));
            num & (1 << (index & 0b111)) > 0
        }
    }

    fn set_at_index(&self, index: u64, allocated: bool) {
        //still ugly code
        unsafe {
            let mut num = *get_at_virtual_addr::<u8>(self.tree_allocator + VirtAddr(index >> 3));
            match allocated {
                true => {
                    num |= 1 << (index & 0b111);
                }
                false => {
                    num &= !(1 << (index & 0b111));
                }
            }
            set_at_virtual_addr(self.tree_allocator + VirtAddr(index >> 3), num);
        }
    }
}

fn find_mem_region_to_shrink(memory_regions: &[&mut limine::MemoryMapEntry], space_needed_bytes: u64) -> usize {
    let mut entry_to_shrink: Option<usize> = None;
    for region in memory_regions.iter().enumerate() {
        if !is_memory_region_usable(region.1) {
            continue;
        }
        let empty_space = region.1.length;
        if empty_space >= space_needed_bytes {
            entry_to_shrink = Some(region.0);
            break;
        }
    }

    entry_to_shrink.unwrap()
}

fn find_max_usable_address(memory_regions: &[&mut limine::MemoryMapEntry]) -> PhysAddr {
    let mut highest = 0;
    for region in memory_regions {
        if is_memory_region_usable(region) {
            highest = region.base + region.length;
        }
    }
    PhysAddr(highest)
}

fn is_memory_region_usable(entry: &limine::MemoryMapEntry) -> bool {
    entry.entry_type == limine::LIMINE_MEMMAP_USABLE// || entry.entry_type == limine::LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE //if we want to use bootloader reclaimable move bootloader structures to our own memory
}

fn get_binary_tree_size(mut n_pages: u64) -> u64 {
    let mut first_bit = 0;
    for i in 0..64 {
        let mask = 1 << (63 - i);
        if n_pages & mask != 0 {
            first_bit = i;
            break;
        }
    }
    let mask = u64::MAX >> (first_bit + 1);
    if n_pages & mask != 0 {
        //needs rounding up
        n_pages = 1 << (63 - first_bit + 1);
    }
    n_pages * 2
}
