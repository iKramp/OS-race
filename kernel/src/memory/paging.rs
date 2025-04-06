use super::{mem_utils::*, physical_allocator};

#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

//first 4 are identical as at power-on/reset
pub enum LiminePat {
    WB = 0,
    WT = 1,
    UCMinus = 2,
    UC = 3,
    WP = 4,
    WC = 5,
}

impl PageTableEntry {
    //creates default entry:
    //present, writeable, not user accessible, not write-through, not cache disabled, not accessed,
    //not dirty, not huge, not global
    pub fn new(address: PhysAddr) -> Self {
        let mut entry = Self((address.0 & 0xF_FFF_FFF_FFF_000) | 0b000000011);
        entry.set_num_of_available_pages(512);
        entry
    }

    pub fn address(&self) -> PhysAddr {
        PhysAddr(self.0 & 0xF_FFF_FFF_FFF_000)
    }

    pub fn present(&self) -> bool {
        self.0 & 0b1 != 0
    }

    pub fn set_present(&mut self, present: bool) {
        const OFFSET: u64 = 0;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn writeable(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    pub fn set_writeable(&mut self, present: bool) {
        const OFFSET: u64 = 1;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn user_accessible(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    pub fn set_user_accessible(&mut self, present: bool) {
        const OFFSET: u64 = 2;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn write_through_caching(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    fn set_write_through_cahcing(&mut self, present: bool) {
        const OFFSET: u64 = 3;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn disable_cache(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    fn set_disable_cahce(&mut self, present: bool) {
        const OFFSET: u64 = 4;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn set_pat(&mut self, pat_val: LiminePat) {
        let (_pat, pcd, pwt) = match pat_val {
            LiminePat::WB => (false, false, false),
            LiminePat::WT => (false, false, true),
            LiminePat::UCMinus => (false, true, false),
            LiminePat::UC => (false, true, true),
            LiminePat::WP => (true, false, false),
            LiminePat::WC => (true, false, true),
        };
        self.set_disable_cahce(pcd);
        self.set_write_through_cahcing(pwt);
        //for now i ignore pat teehee :3
        //pat bit depends on if it's a page directory or page table. Can be checked with huge
        //table, but huge-huge tables (1GB) also have huge tables, and don't have pat bit

        PageTree::reload();
    }

    pub fn accessed(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub fn set_accessed(&mut self, present: bool) {
        const OFFSET: u64 = 5;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn dirty(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    pub fn set_dirty(&mut self, present: bool) {
        const OFFSET: u64 = 6;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn huge_page(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn set_huge_page(&mut self, present: bool) {
        const OFFSET: u64 = 7;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn global(&self) -> bool {
        self.0 & (1 << 8) != 0
    }

    pub fn set_global(&mut self, present: bool) {
        const OFFSET: u64 = 8;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((present as u64) << OFFSET);
    }

    pub fn num_of_available_pages(&self) -> u64 {
        (self.0 >> 52) & 0b1111111111
    }

    fn set_num_of_available_pages(&mut self, num: u64) {
        const MASK: u64 = !(0b11111111111_u64 << 52);
        const NUM_MASK: u64 = !MASK;
        self.0 = (self.0 & MASK) | ((num << 52) & NUM_MASK);
    }

    pub fn decrease_available(&mut self) {
        self.0 -= 1 << 52;
    }

    pub fn increase_available(&mut self) {
        self.0 += 1 << 52;
    }

    pub fn no_execute(&self) -> bool {
        self.0 & (1 << 63) != 0
    }

    pub fn set_no_execute(&mut self, no_execute: bool) {
        const OFFSET: u64 = 63;
        const MASK: u64 = 1 << OFFSET;
        const INVERSE_MASK: u64 = MASK ^ u64::MAX;
        self.0 = (self.0 & INVERSE_MASK) | ((no_execute as u64) << OFFSET);
    }
}

impl core::fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Format the output
        f.debug_struct("PageTableEntry")
            .field("present", &self.present())
            .field("address", &self.address())
            .field("available pages", &self.num_of_available_pages())
            .field("huge page", &self.huge_page())
            .field("no execute", &self.no_execute())
            .field("writeable", &self.writeable())
            .field("write through", &self.write_through_caching())
            .field("disable cache", &self.disable_cache())
            .field("user accessible", &self.user_accessible())
            .field("accessed", &self.accessed())
            .field("dirty", &self.dirty())
            .field("global", &self.global())
            .finish()
    }
}

#[repr(align(4096))]
#[derive(Debug)]
struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = PageTableEntry(0);
        }
    }

    ///returns entry index at which a page is available. If no such address exists, it panics
    pub fn get_available_entry(&self) -> usize {
        for entry in self.entries.iter().enumerate() {
            if !entry.1.present() || (!entry.1.huge_page() && entry.1.num_of_available_pages() > 0) {
                return entry.0;
            }
        }
        panic!("could not find available virtual page");
    }

    ///returns the address after which the requested number of pages are available.
    ///If such an address doesn't exist the function panics, because in reality that's pretty much
    ///impossible. Computers will run out of memory before that happens
    ///# Safety
    ///This function must be called with a valid level. External callers should always use 4
    ///number of pages requested cannot be more than 512
    pub unsafe fn get_available_entry_pages(&self, level: u64, pages: u64) -> u64 {
        debug_assert!(pages <= 512);
        for entry in self.entries.iter().enumerate() {
            if !entry.1.present() {
                return (entry.0 as u64) << (3 + level * 9);
            }
            if entry.1.present() && (!entry.1.huge_page() && entry.1.num_of_available_pages() >= pages) {
                let lower_table = unsafe { get_at_physical_addr::<PageTable>(entry.1.address()) };
                let addr = if level == 2 {
                    lower_table.get_available_entry_level_1_pages(pages)
                } else {
                    unsafe { lower_table.get_available_entry_pages(level - 1, pages) }
                };
                return ((entry.0 as u64) << (3 + level * 9)) + addr;
            }
        }
        panic!("could not find available virtual page");
    }

    fn get_available_entry_level_1(&self) -> usize {
        for entry in self.entries.iter().enumerate() {
            if !entry.1.present() {
                return entry.0;
            }
        }
        panic!("could not find available virtual page");
    }

    ///returns the address after which the requested number of pages are available.
    fn get_available_entry_level_1_pages(&self, pages: u64) -> u64 {
        for entries in self.entries.windows(pages as usize).enumerate() {
            if entries.1.iter().all(|entry| !entry.present()) {
                return (entries.0 as u64) << 12;
            }
        }
        panic!("could not find available virtual page");
    }

    pub fn allocate_any(&mut self) -> VirtAddr {
        unsafe {
            let frame_addr = physical_allocator::allocate_frame();
            self.mmap_any(frame_addr)
        }
    }

    ///maps some available virtual address to the given physical address. Physical address must be
    ///marked as used
    ///# Safety
    ///Physical address must be marked as used by an external actor
    pub unsafe fn mmap_any(&mut self, physical_address: PhysAddr) -> VirtAddr {
        debug_assert!(physical_allocator::is_frame_allocated(physical_address));
        let mut address = 0;
        unsafe { self.allocate_4_to_2(4, &mut address, physical_address) };
        if address & (1 << 47) != 0 {
            address += 0xFFFF << 48; //sign extension
        }
        VirtAddr(address)
    }

    ///# Safety
    ///Virtual address must not yet be in use by this page tree
    pub unsafe fn allocate(&mut self, virtual_address: VirtAddr) {
        let frame_addr = physical_allocator::allocate_frame();
        unsafe { self.mmap(virtual_address, frame_addr) }
    }

    ///# Seafety
    ///Physical addresses from physical_address to physical_address + num must already be marked as
    ///used, and not yet mapped
    pub unsafe fn mmap_contigious_any(&mut self, num: u64, physical_address: PhysAddr) -> VirtAddr {
        let address = unsafe { self.get_available_entry_pages(4, num) };
        for i in 0..num {
            assert!(
                physical_allocator::is_frame_allocated(physical_address + PhysAddr(i * 4096)),
                "memory space must already be allocated"
            );
            unsafe {
                self.mmap(VirtAddr(address + i * 4096), physical_address + PhysAddr(i * 4096));
            }
        }
        VirtAddr(address)
    }

    ///maps the given virtual address to the given physical address. Physical address must be
    ///marked as used
    ///# Safety
    ///physical address must be marked as used by an external actor. Virtual address must
    ///not yet be in use by this page tree
    pub unsafe fn mmap(&mut self, virtual_address: VirtAddr, physical_address: PhysAddr) {
        debug_assert!(physical_allocator::is_frame_allocated(physical_address));
        unsafe {
            self.allocate_4_to_2_virtual(4, virtual_address, physical_address);
        }
    }

    ///returns if that page table has less available spaces
    unsafe fn allocate_4_to_2(&mut self, level: u64, address: &mut u64, physical_address: PhysAddr) -> bool {
        let index_of_available = self.get_available_entry();

        *address += (index_of_available as u64) << (3 + level * 9);
        let entry = &mut self.entries[index_of_available];

        if !entry.present() {
            let frame_addr = physical_allocator::allocate_frame();
            let page_table = unsafe { get_at_physical_addr::<PageTable>(frame_addr) };
            page_table.clear();
            let temp_entry = PageTableEntry::new(frame_addr);
            *entry = temp_entry;
        }

        let lower_page_table = unsafe { get_at_physical_addr::<PageTable>(entry.address()) };
        let lower_less_available = if level == 2 {
            unsafe { lower_page_table.allocate_level_1(address, physical_address) };
            true
        } else {
            unsafe { lower_page_table.allocate_4_to_2(level - 1, address, physical_address) }
        };
        if lower_less_available {
            entry.decrease_available();
        }
        entry.num_of_available_pages() == 0
    }

    ///#Safety: Virtual address must not yet be in use by this page tree. Physical address must be
    ///marked as used
    unsafe fn allocate_4_to_2_virtual(&mut self, level: u64, address: VirtAddr, physical_address: PhysAddr) -> bool {
        let entry = self.get_page_table_entry_on_level(address, level);
        if !entry.present() {
            let frame_addr = physical_allocator::allocate_frame();
            let page_table = unsafe { get_at_physical_addr::<PageTable>(frame_addr) };
            page_table.clear();
            let temp_entry = PageTableEntry::new(frame_addr);
            *entry = temp_entry;
            assert_eq!(temp_entry.0, entry.0);
            assert_eq!(temp_entry.address(), entry.address());
        }

        let lower_less_available = unsafe {
            let lower_page_table = get_at_physical_addr::<PageTable>(entry.address());
            if level == 2 {
                lower_page_table.allocate_level_1_virtual(address, physical_address);
                true
            } else {
                lower_page_table.allocate_4_to_2_virtual(level - 1, address, physical_address)
            }
        };
        if lower_less_available {
            entry.decrease_available();
        }
        entry.num_of_available_pages() == 0
    }

    ///# Safety
    ///This level MUST have empty address slot. This must be ensured by higher levels
    unsafe fn allocate_level_1(&mut self, address: &mut u64, physical_address: PhysAddr) {
        let index_of_available = self.get_available_entry_level_1();
        *address += (index_of_available as u64) << 12;
        let entry = &mut self.entries[index_of_available];

        *entry = PageTableEntry::new(physical_address);
    }

    unsafe fn allocate_level_1_virtual(&mut self, virtual_address: VirtAddr, physical_address: PhysAddr) {
        let entry = self.get_page_table_entry_on_level(virtual_address, 1);
        debug_assert!(
            !entry.present(),
            "requested virtual address {:#x?} already used. Entry: {:#x?}",
            virtual_address,
            entry
        );

        *entry = PageTableEntry::new(physical_address);
    }

    //returns if there was no space but now there is
    pub unsafe fn deallocate(&mut self, address: VirtAddr, level: u64) -> bool {
        let entry = &mut self.entries[(address.0 >> (3 + level * 9) & 0b111_111_111) as usize];
        debug_assert!(entry.present());
        if level == 1 {
            entry.set_present(false);
            unsafe { physical_allocator::deallocate_frame(entry.address()) };
            return true;
        }
        if entry.present() && entry.huge_page() {
            entry.set_present(false);
            dealloc_huge_page(entry, level);
            return true;
        }
        unsafe {
            let lower_level_table = get_at_physical_addr::<PageTable>(entry.address());
            let more_space = lower_level_table.deallocate(address, level - 1);
            if !more_space {
                return false;
            }
        }
        entry.increase_available();
        //if it was 0 before, this entry was not available but now it is
        entry.num_of_available_pages() == 1
    }

    //returns if there was no space but now there is
    pub unsafe fn unmap(&mut self, address: VirtAddr, level: u64) -> bool {
        let entry = &mut self.entries[(address.0 >> (3 + level * 9) & 0b111_111_111) as usize];
        debug_assert!(entry.present());
        if level == 1 {
            entry.set_present(false);
            return true;
        }
        if entry.present() && entry.huge_page() {
            entry.set_present(false);
            return true;
        }
        unsafe {
            let lower_level_table = get_at_physical_addr::<PageTable>(entry.address());
            let more_space = lower_level_table.unmap(address, level - 1);
            if !more_space {
                return false;
            }
        }
        entry.increase_available();
        //if it was 0 before, this entry was not available but now it is
        entry.num_of_available_pages() == 1
    }

    pub fn set_num_of_available_spaces(&mut self, level: u64) -> u64 {
        let mut sum = 0;
        for entry in &mut self.entries {
            if !entry.present() {
                sum += 1;
                continue;
            }
            if level == 1 || entry.huge_page() {
                continue;
            }
            unsafe {
                let lower_level_page = get_at_physical_addr::<PageTable>(entry.address());
                let lower_available = lower_level_page.set_num_of_available_spaces(level - 1);
                entry.set_num_of_available_pages(lower_available);
                if lower_available > 0 {
                    sum += 1;
                }
            }
        }
        sum
    }

    fn get_num_allocated_spaces(&self, level: u64) -> u64 {
        let mut sum = 0;
        for entry in &self.entries {
            if !entry.present() {
                continue;
            }
            if level == 1 || entry.huge_page() {
                sum += 1;
                continue;
            }
            unsafe {
                let lower_level_page = get_at_physical_addr::<PageTable>(entry.address());
                let lower_available = lower_level_page.get_num_allocated_spaces(level - 1);
                sum += lower_available;
            }
        }
        sum
    }

    fn get_page_table_entry(&mut self, addr: VirtAddr, level: u64) -> &mut PageTableEntry {
        unsafe {
            let entry = (addr.0 >> (12 + 9 * (level - 1))) & 0x1FF;
            let entry = &mut self.entries[entry as usize];
            if level == 1 {
                entry
            } else {
                let lower_table = get_at_physical_addr::<PageTable>(entry.address());
                lower_table.get_page_table_entry(addr, level - 1)
            }
        }
    }

    fn get_page_table_entry_on_level(&mut self, addr: VirtAddr, level: u64) -> &mut PageTableEntry {
        let entry = (addr.0 >> (12 + 9 * (level - 1))) & 0x1FF;
        &mut self.entries[entry as usize]
    }
}

fn dealloc_huge_page(entry: &PageTableEntry, level: u64) {
    #[cfg(debug_assertions)]
    assert!(level == 2 || level == 3);

    let physical_address = entry.address();
    let num_to_dealloc = 512 * if level == 3 { 512 } else { 1 };
    for j in 0..num_to_dealloc {
        unsafe {
            physical_allocator::deallocate_frame(physical_address + PhysAddr(j * 4096));
        }
    }
}

pub struct PageTree {
    pub level_4_table: PhysAddr,
}

impl PageTree {
    pub fn new(level_4_table: PhysAddr) -> Self {
        Self { level_4_table }
    }

    ///This function walks the page table and sets the number of available spaces in the lower
    ///level pages. It also maps addr 0 as user inaccessible, not writable and not executable.
    ///Kernel can still read, but by mapping it to physical address 0 and not using it it's fine
    pub fn init(&mut self) {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            level_4_table.set_num_of_available_spaces(4);
            level_4_table.mmap(VirtAddr(0), PhysAddr(0));
            let entry = level_4_table.get_page_table_entry(VirtAddr(0), 4);
            entry.set_user_accessible(false);
            entry.set_writeable(false);
            entry.set_no_execute(true);
        }
    }

    pub fn get_num_allocated_pages(&self) -> u64 {
        let level_4_table = unsafe { get_at_physical_addr::<PageTable>(self.level_4_table) };
        level_4_table.get_num_allocated_spaces(4)
    }

    pub fn get_level4_addr() -> PhysAddr {
        let mut level_4_table = PhysAddr(0);
        unsafe {
            core::arch::asm!(
                "mov {}, cr3",
                out(reg) level_4_table.0,
            );
        }
        level_4_table
    }

    pub fn reload() {
        unsafe {
            let level_4_table = Self::get_level4_addr();
            core::arch::asm!(
                "mov cr3, {}",
                in(reg) level_4_table.0,
            );
        }
    }

    pub fn get_page_table_entry_mut(&mut self, addr: std::mem_utils::VirtAddr) -> &mut PageTableEntry {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            level_4_table.get_page_table_entry(VirtAddr(addr.0 & !0xFFF), 4)
        }
    }
}

impl std::PageAllocator for PageTree {
    fn allocate(&mut self, physical_address: Option<PhysAddr>) -> std::mem_utils::VirtAddr {
        //TODO:, make mmap and such methods here instead of Options
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            match physical_address {
                None => level_4_table.allocate_any(),
                Some(physical_address) => level_4_table.mmap_any(physical_address),
            }
        }
    }

    fn allocate_set_virtual(&mut self, physical_address: Option<PhysAddr>, virtual_address: std::mem_utils::VirtAddr) {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            match physical_address {
                None => level_4_table.allocate(virtual_address),
                Some(physical_address) => level_4_table.mmap(virtual_address, physical_address),
            }
        }
    }

    fn deallocate(&mut self, addr: std::mem_utils::VirtAddr) {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            level_4_table.deallocate(addr, 4);
        }
    }

    fn unmap(&mut self, addr: std::mem_utils::VirtAddr) {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            level_4_table.unmap(addr, 4);
        }
    }

    fn allocate_contigious(&mut self, num: u64, physical_address: Option<PhysAddr>) -> std::mem_utils::VirtAddr {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            match physical_address {
                None => {
                    let addr = level_4_table.get_available_entry_pages(4, num);
                    for i in 0..num {
                        level_4_table.allocate(VirtAddr(addr + i * 4096));
                    }
                    VirtAddr(addr)
                }
                Some(physical_address) => level_4_table.mmap_contigious_any(num, physical_address),
            }
        }
    }

    fn mmap_contigious(&mut self, physical_addresses: &[PhysAddr]) -> std::mem_utils::VirtAddr {
        unsafe {
            let level_4_table = get_at_physical_addr::<PageTable>(self.level_4_table);
            let addr = level_4_table.get_available_entry_pages(4, physical_addresses.len() as u64);
            for i in 0..physical_addresses.len() {
                level_4_table.mmap(VirtAddr(addr + i as u64 * 4096), physical_addresses[i]);
            };
            VirtAddr(addr)
        }
    }

    fn find_contigious_pages(&mut self, n_pages: usize) -> std::mem_utils::VirtAddr {
        let level_4_table = unsafe { get_at_physical_addr::<PageTable>(self.level_4_table) };
        unsafe { VirtAddr(level_4_table.get_available_entry_pages(4, n_pages as u64)) }
    }
}
