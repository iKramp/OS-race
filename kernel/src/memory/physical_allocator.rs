use super::mem_utils::*;
use crate::println;
use bootloader_api::{info::MemoryRegionKind, BootInfo};

pub static mut BUDDY_ALLOCATOR: BuyddyAllocator = BuyddyAllocator {
    n_pages: 0,
    binary_tree_size: 0,
    tree_allocator: VirtAddr(0),
};

pub struct BuyddyAllocator {
    n_pages: u64,
    binary_tree_size: u64,
    tree_allocator: VirtAddr,
}

impl BuyddyAllocator {
    pub fn init(boot_info: &'static mut BootInfo) {
        let memory_regions = &mut boot_info.memory_regions as &'static mut [bootloader_api::info::MemoryRegion];
        let n_pages = find_max_usable_address(memory_regions).0 >> 12;

        let binary_tree_size_elements = get_binary_tree_size(n_pages);

        // div by 8 for 8 bits in a byte  (also rounded up), times 2 for binary tree
        let space_needed_bytes = (binary_tree_size_elements + 7) / 8;
        let entry_to_shrink = find_mem_region_to_shrink(memory_regions, space_needed_bytes);

        println!("{binary_tree_size_elements:?}");
        println!("{entry_to_shrink:?}");

        let tree_allocator = PhysAddr(memory_regions[entry_to_shrink].start);
        for i in 0..space_needed_bytes {
            //set all bits to 1 to set everything as used
            unsafe {
                set_at_physical_addr(tree_allocator + PhysAddr(i), 0xFF_u8);
            }
        }
        let mut size_to_shrink = space_needed_bytes & !0xFFF;
        if space_needed_bytes & 0xFFF > 0 {
            size_to_shrink += 0x1000;
        }
        //we essentially round up to a whole page
        memory_regions[entry_to_shrink].start += size_to_shrink;
        let allocator = Self {
            n_pages,
            binary_tree_size: binary_tree_size_elements,
            tree_allocator: translate_phys_virt_addr(tree_allocator),
        };
        for entry in memory_regions {
            println!("0x{entry:x?}");
            if entry.kind != bootloader_api::info::MemoryRegionKind::Usable {
                continue;
            }
            for addr in (entry.start..entry.end).step_by(4096) {
                allocator.mark_addr(PhysAddr(addr), false);
            }
        }
        unsafe { BUDDY_ALLOCATOR = allocator }
    }

    pub fn deallocate_frame(&mut self, addr: PhysAddr) {
        self.mark_addr(addr, false)
    }

    pub fn allocate_frame(&mut self) -> PhysAddr {
        let index = self.find_empty_page();
        self.mark_index(index, true);
        PhysAddr((index - self.binary_tree_size / 2) * 4096)
    }

    fn find_empty_page(&self) -> u64 {
        assert!(!self.get_at_index(1), "root node of physical memory allocator is filled");
        self.find_empty_page_recursively(1)
    }

    fn find_empty_page_recursively(&self, curr_index: u64) -> u64 {
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
            self.find_empty_page_recursively(curr_index * 2)
        } else {
            self.find_empty_page_recursively(curr_index * 2 + 1)
        }
    }

    fn mark_addr(&self, addr: PhysAddr, allocated: bool) {
        #[cfg(debug_assertions)]
        assert!(
            addr.0 & 0xFFF == 0,
            "error in mark_addr at addr {} and allocated {}",
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

fn find_mem_region_to_shrink(memory_regions: &[bootloader_api::info::MemoryRegion], space_needed_bytes: u64) -> usize {
    let mut entry_to_shrink: Option<usize> = None;
    for region in memory_regions.iter().enumerate() {
        if region.1.kind != bootloader_api::info::MemoryRegionKind::Usable {
            continue;
        }
        let empty_space = region.1.end - region.1.start;
        if empty_space >= space_needed_bytes {
            entry_to_shrink = Some(region.0);
            break;
        }
    }

    entry_to_shrink.unwrap()
}

fn find_max_usable_address(memory_regions: &[bootloader_api::info::MemoryRegion]) -> PhysAddr {
    let mut highest = 0;
    for region in memory_regions {
        if region.kind == MemoryRegionKind::Usable {
            highest = region.end;
        }
    }
    PhysAddr(highest)
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
